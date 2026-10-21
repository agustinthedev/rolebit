import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("Rolebit application shell", () => {
  it("loads the local overview and primary navigation", async () => {
    render(<App />);
    expect(screen.getByText("Opening your local library…")).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "Good morning." })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /All Opportunities/i })).toBeInTheDocument();
    expect(screen.getByText("Browser preview · demo data")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Profile" })).toBeInTheDocument();
  });

  it("opens the local profile workspace", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "Good morning." });
    await user.click(screen.getByRole("button", { name: "Profile" }));
    expect(await screen.findByRole("heading", { name: "Personal details" })).toBeInTheDocument();
    expect(screen.getAllByText("Local & private").length).toBeGreaterThan(0);
  });
});
