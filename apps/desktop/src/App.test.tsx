import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("desktop shell", () => {
  it("renders required minimum loop surfaces", async () => {
    render(<App />);
    expect(await screen.findByText("Project picker")).toBeInTheDocument();
    expect(screen.getByText("Agent picker")).toBeInTheDocument();
    expect(screen.getByText("Profile picker")).toBeInTheDocument();
    expect(screen.getByText("Session Status")).toBeInTheDocument();
    expect(screen.getByText("Violation View")).toBeInTheDocument();
    expect(screen.getByText("Audit Stream")).toBeInTheDocument();
  });
});
