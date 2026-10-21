import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("Rolebit application shell", () => {
  it("loads the local overview and primary navigation", async () => {
    render(<App />);
    expect(screen.getByText("Opening your local library…")).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "Good morning." })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /All Opportunities/i })).toBeInTheDocument();
    expect(screen.getByText("Browser preview · demo data")).toBeInTheDocument();
  });
});
