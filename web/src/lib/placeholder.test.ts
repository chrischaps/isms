import { expect, test } from "vitest";
import { appName } from "./placeholder";

test("app is named", () => {
  expect(appName()).toBe("Isms");
});
