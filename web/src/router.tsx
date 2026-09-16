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
import { Gallery } from "./screens/Gallery";
import { Login } from "./screens/Login";
import { PlanScreen } from "./screens/Plan";
import { Societies } from "./screens/Societies";
import { SocietyHome, SocietyShell } from "./screens/Society";
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

const routeTree = rootRoute.addChildren([
  indexRoute,
  loginRoute,
  galleryRoute,
  societyRoute.addChildren([societyHomeRoute, societyWorkRoute, societyPlanRoute]),
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
