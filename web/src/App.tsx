import { appName } from "./lib/placeholder";

export default function App() {
  return (
    <main style={{ fontFamily: "system-ui", padding: "2rem" }}>
      <h1>{appName()}</h1>
      <p>The economy is the game. Web client arrives in Phase 1 (S1.7).</p>
    </main>
  );
}
