import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("desktop shell", () => {
  it("shows blocked file read explanation after launch", async () => {
    render(<App />);

    expect(await screen.findByText("Windows Safe")).toBeInTheDocument();
    const launch = await screen.findByRole("button", { name: "Launch session" });
    fireEvent.click(launch);

    expect(await screen.findByText("read blocked")).toBeInTheDocument();
    expect(
      screen.getByText(/Blocked read on C:\\Users\\dev\\.ssh\\config/),
    ).toBeInTheDocument();
    expect(screen.getByText("Capability Snapshot")).toBeInTheDocument();
  });
});
