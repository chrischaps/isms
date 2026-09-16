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
import { Contracts } from "./screens/Contracts";
import { Gallery } from "./screens/Gallery";
import { Login } from "./screens/Login";
import { Market } from "./screens/Market";
import { Org } from "./screens/Org";
import { Orgs } from "./screens/Orgs";
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

const routeTree = rootRoute.addChildren([
  indexRoute,
  loginRoute,
  galleryRoute,
  societyRoute.addChildren([
    societyHomeRoute,
    societyWorkRoute,
    societyPlanRoute,
    societyMarketRoute,
    societyBookRoute,
    societyOrgsRoute,
    societyOrgRoute,
    societyContractsRoute,
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
