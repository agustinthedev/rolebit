import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SearchListEditor } from "./SearchListEditor";

describe("SearchListEditor", () => {
  it("adds and edits strategy entries", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<SearchListEditor kind="role" title="Roles" description="Search terms" items={[{ value: "QA", enabled: true }]} onChange={onChange} />);
    await user.click(screen.getByRole("button", { name: "Add role" }));
    expect(onChange).toHaveBeenCalledWith([{ value: "QA", enabled: true }, { value: "", enabled: true }]);
    await user.clear(screen.getByLabelText("Roles item 1"));
    await user.type(screen.getByLabelText("Roles item 1"), "SDET");
    expect(onChange).toHaveBeenCalled();
  });
});
