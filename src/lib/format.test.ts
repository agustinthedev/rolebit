import { describe, expect, it } from "vitest";
import { plainText } from "./format";

describe("plainText", () => {
  it("strips regular HTML and decodes entities", () => {
    expect(plainText("<p>Build <strong>quality</strong> &amp; reliable systems.</p>")).toBe("Build quality & reliable systems.");
  });

  it("strips HTML that was encoded before being stored", () => {
    expect(plainText("&lt;p style=&quot;min-height:1.5em&quot;&gt;Join Hire Hangar &amp; teams.&lt;/p&gt;")).toBe("Join Hire Hangar & teams.");
  });

  it("keeps paragraphs and list bullets", () => {
    expect(plainText("<p>Role overview</p><p>Key responsibilities:</p><ul><li>Own delivery</li><li>Improve quality</li></ul>")).toBe("Role overview\n\nKey responsibilities:\n\n• Own delivery\n• Improve quality");
  });
});
