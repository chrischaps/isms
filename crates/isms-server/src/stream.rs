//! `GET /s/{id}/stream` (TDD 10.3): the live event stream over WebSocket,
//! filtered per viewer, with `TickResolved` reduced to the viewer's own deltas
//! and the public aggregates. Clients use it to invalidate what they cache.

use crate::actor::{Batch, SocietyHandle};
use crate::api::{citizen_in, society, touch_presence};
use crate::auth::Auth;
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, clock_of};
use crate::viewer::Viewer;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use isms_api_types::society::{EventRef, StreamFrame};
use isms_core::ids::CitizenId;
use tokio::sync::broadcast::error::RecvError;

#[utoipa::path(get, path = "/s/{id}/stream", summary = "WebSocket: every event you may see, as it is committed",
    params(("id" = i64, Path, description = "Society id")),
    responses((status = 101, description = "Switching to WebSocket"), (status = 403, body = isms_api_types::Problem)),
    security(("session" = []), ("api_key" = [])))]
pub async fn stream(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<i64>,
    ws: WebSocketUpgrade,
) -> ApiResult<Response> {
    let entry = society(&state, id)?;
    let row = citizen_in(&state, &auth, id)
        .await?
        .ok_or_else(|| ApiError::Forbidden("join this society first".into()))?;
    touch_presence(&state, &auth, &entry).await?;
    let me = CitizenId(u32::try_from(row.citizen_id).unwrap_or(u32::MAX));
    let handle = entry.handle.clone();
    Ok(ws.on_upgrade(move |socket| run(socket, handle, me)))
}

async fn frame(
    handle: &SocietyHandle,
    me: CitizenId,
    batch: Option<&Batch>,
    lagged: Option<u64>,
) -> String {
    let world = handle.world.read().await;
    let viewer = Viewer::new(&world, Some(me));
    let events = batch.map_or_else(Vec::new, |b| {
        b.events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let payload = viewer.view_event(&world, e)?;
                Some(EventRef {
                    seq: b.first_seq + i64::try_from(i).unwrap_or(0),
                    tick: b.tick,
                    cycle: b.cycle,
                    epoch: world.meta.epoch,
                    kind: e.kind().to_owned(),
                    payload,
                })
            })
            .collect()
    });
    let f = StreamFrame {
        clock: clock_of(&world),
        events,
        lagged,
    };
    serde_json::to_string(&f).unwrap_or_default()
}

async fn run(mut socket: WebSocket, handle: SocietyHandle, me: CitizenId) {
    let mut rx = handle.subscribe();
    // A first frame so the client knows the clock at subscription time.
    let hello = frame(&handle, me, None, None).await;
    if socket.send(Message::Text(hello.into())).await.is_err() {
        return;
    }
    loop {
        tokio::select! {
            batch = rx.recv() => {
                let text = match batch {
                    // Frames with nothing this viewer may see are still sent: the clock moved.
                    Ok(b) => frame(&handle, me, Some(&b), None).await,
                    Err(RecvError::Lagged(n)) => frame(&handle, me, None, Some(n)).await,
                    Err(RecvError::Closed) => break,
                };
                if socket.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}
