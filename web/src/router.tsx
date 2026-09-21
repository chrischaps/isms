// Routes (TDD 11): a landing with the society list, sign-in, the society
// shell whose nav is built from capabilities and lexicon, and a component
// gallery for visual checks. Code-based TanStack Router, no file plugin.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  Link,
  Outlet,
  RouterProvider,
  createRootRoute,
  createRoute,
  createRouter,
  useParams,
} from "@tanstack/react-router";
import { Admin } from "./screens/Admin";
import { PublicArchives, SocietyArchives } from "./screens/Archives";
import { Assembly } from "./screens/Assembly";
import { Coordinator } from "./screens/roles/Coordinator";
import { Contracts } from "./screens/Contracts";
import { EventScreen } from "./screens/Event";
import { Gallery } from "./screens/Gallery";
import { LedgerScreen } from "./screens/Ledger";
import { Login } from "./screens/Login";
import { Market } from "./screens/Market";
import { Org } from "./screens/Org";
import { Orgs } from "./screens/Orgs";
import { PlanScreen } from "./screens/Plan";
import { Profile } from "./screens/Profile";
import { PublicSocieties, PublicSociety } from "./screens/Public";
import { Societies } from "./screens/Societies";
import { SocietyHome, SocietyShell } from "./screens/Society";
import { SocietyScreen } from "./screens/SocietyScreen";
import { Store } from "./screens/Store";
import { Talk } from "./screens/Talk";
import { Work } from "./screens/Work";

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, staleTime: 5_000 } },
});

const rootRoute = createRootRoute({
  component: () => (
    <div className="mx-auto max-w-5xl px-4 py-6">
      <Outlet />
    </div>
  ),
  notFoundComponent: () => (
    <p>
      Nothing here. <Link to="/">Home</Link>
    </p>
  ),
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: Societies,
});

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  component: Login,
});

const galleryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/gallery",
  component: Gallery,
});

function SocietyRoute() {
  const { id } = useParams({ from: "/s/$id" });
  return <SocietyShell id={Number(id)} />;
}

function SocietyHomeRoute() {
  const { id } = useParams({ from: "/s/$id/" });
  return <SocietyHome id={Number(id)} />;
}

function SocietyWorkRoute() {
  const { id } = useParams({ from: "/s/$id/work" });
  return <Work id={Number(id)} />;
}

function SocietyPlanRoute() {
  const { id } = useParams({ from: "/s/$id/plan" });
  return <PlanScreen id={Number(id)} />;
}

function SocietyMarketRoute() {
  const { id } = useParams({ from: "/s/$id/market" });
  return <Market id={Number(id)} />;
}

function SocietyBookRoute() {
  const { id, instrument } = useParams({ from: "/s/$id/market/$instrument" });
  return <Market id={Number(id)} instrument={instrument} />;
}

function SocietyOrgsRoute() {
  const { id } = useParams({ from: "/s/$id/orgs" });
  return <Orgs id={Number(id)} />;
}

function SocietyOrgRoute() {
  const { id, oid } = useParams({ from: "/s/$id/orgs/$oid" });
  return <Org id={Number(id)} oid={Number(oid)} />;
}

function SocietyContractsRoute() {
  const { id } = useParams({ from: "/s/$id/contracts" });
  return <Contracts id={Number(id)} />;
}

function SocietySocietyRoute() {
  const { id } = useParams({ from: "/s/$id/society" });
  return <SocietyScreen id={Number(id)} />;
}

function SocietyStoreRoute() {
  const { id } = useParams({ from: "/s/$id/store" });
  return <Store id={Number(id)} />;
}

function SocietyLedgerRoute() {
  const { id } = useParams({ from: "/s/$id/ledger" });
  return <LedgerScreen id={Number(id)} />;
}

function SocietyAssemblyRoute() {
  const { id } = useParams({ from: "/s/$id/assembly" });
  return <Assembly id={Number(id)} />;
}

function SocietyCoordinatorRoute() {
  const { id } = useParams({ from: "/s/$id/coordinator" });
  return <Coordinator id={Number(id)} />;
}

function SocietyTalkRoute() {
  const { id } = useParams({ from: "/s/$id/talk" });
  return <Talk id={Number(id)} />;
}

function SocietyChannelRoute() {
  const { id, channel } = useParams({ from: "/s/$id/talk/$channel" });
  return <Talk id={Number(id)} channel={channel} />;
}

function SocietyEventRoute() {
  const { id, seq } = useParams({ from: "/s/$id/events/$seq" });
  return <EventScreen id={Number(id)} seq={Number(seq)} />;
}

function PublicSocietyRoute() {
  const { id } = useParams({ from: "/public/s/$id" });
  return <PublicSociety id={Number(id)} />;
}

function PublicArchivesRoute() {
  const { id } = useParams({ from: "/public/s/$id/archives" });
  return <PublicArchives id={Number(id)} />;
}

function SocietyArchivesRoute() {
  const { id } = useParams({ from: "/s/$id/archives" });
  return <SocietyArchives id={Number(id)} />;
}

const adminRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/admin",
  component: Admin,
});

const profileRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/profile",
  component: Profile,
});

const publicRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/public",
  component: PublicSocieties,
});

const publicSocietyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/public/s/$id",
  component: PublicSocietyRoute,
});

const publicArchivesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/public/s/$id/archives",
  component: PublicArchivesRoute,
});

const societyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/s/$id",
  component: SocietyRoute,
});

const societyHomeRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/",
  component: SocietyHomeRoute,
});

const societyWorkRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/work",
  component: SocietyWorkRoute,
});

const societyPlanRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/plan",
  component: SocietyPlanRoute,
});

const societyMarketRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/market",
  component: SocietyMarketRoute,
});

const societyBookRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/market/$instrument",
  component: SocietyBookRoute,
});

const societyOrgsRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/orgs",
  component: SocietyOrgsRoute,
});

const societyOrgRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/orgs/$oid",
  component: SocietyOrgRoute,
});

const societyContractsRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/contracts",
  component: SocietyContractsRoute,
});

const societySocietyRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/society",
  component: SocietySocietyRoute,
});

const societyStoreRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/store",
  component: SocietyStoreRoute,
});

const societyLedgerRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/ledger",
  component: SocietyLedgerRoute,
});

const societyAssemblyRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/assembly",
  component: SocietyAssemblyRoute,
});

const societyCoordinatorRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/coordinator",
  component: SocietyCoordinatorRoute,
});

const societyTalkRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/talk",
  component: SocietyTalkRoute,
});

const societyChannelRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/talk/$channel",
  component: SocietyChannelRoute,
});

const societyEventRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/events/$seq",
  component: SocietyEventRoute,
});

const societyArchivesRoute = createRoute({
  getParentRoute: () => societyRoute,
  path: "/archives",
  component: SocietyArchivesRoute,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  loginRoute,
  galleryRoute,
  profileRoute,
  adminRoute,
  publicRoute,
  publicSocietyRoute,
  publicArchivesRoute,
  societyRoute.addChildren([
    societyHomeRoute,
    societyWorkRoute,
    societyPlanRoute,
    societyMarketRoute,
    societyBookRoute,
    societyOrgsRoute,
    societyOrgRoute,
    societyContractsRoute,
    societySocietyRoute,
    societyStoreRoute,
    societyLedgerRoute,
    societyAssemblyRoute,
    societyCoordinatorRoute,
    societyTalkRoute,
    societyChannelRoute,
    societyEventRoute,
    societyArchivesRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}
