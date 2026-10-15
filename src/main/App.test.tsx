import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";

test("first run walks through welcome, model and try-it", async () => {
  render(<App />);
  expect(await screen.findByRole("heading", { name: "Make anything readable" })).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Get started" }));
  expect(screen.getByRole("heading", { name: "Download a model" })).toBeInTheDocument();
  expect(screen.getByText("Gemma 4 E2B")).toBeInTheDocument();
  // The catalog shows the fit hint (models found on the computer may show it too, once found).
  expect(screen.getAllByText("Fits easily").length).toBeGreaterThan(0);
  // Next is disabled until a model is chosen or downloading.
  expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
});
