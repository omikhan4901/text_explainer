import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";

test("first run walks through welcome, model and try-it", async () => {
  render(<App />);
  expect(await screen.findByRole("heading", { name: "Make anything readable" })).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Get started" }));
  expect(screen.getByRole("heading", { name: "Download a model" })).toBeInTheDocument();
  expect(screen.getByText("Qwen3.5 4B")).toBeInTheDocument();
  expect(screen.getByText("Fits easily")).toBeInTheDocument();
  // Next is disabled until a model is chosen or downloading.
  expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
});
