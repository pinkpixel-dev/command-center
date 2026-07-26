import { useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { api } from "../lib/ipc";
import type { Collection } from "../lib/types";
import { CollectionField } from "./CollectionField";

const collections: Collection[] = [
  {
    id: 7,
    name: "Docker",
    description: "",
    commandCount: 3,
    createdAt: "2026-07-01T00:00:00Z",
    updatedAt: "2026-07-01T00:00:00Z",
  },
  {
    id: 8,
    name: "Git",
    description: "",
    commandCount: 2,
    createdAt: "2026-07-01T00:00:00Z",
    updatedAt: "2026-07-01T00:00:00Z",
  },
];

function StatefulField({ initial = [] }: { initial?: number[] }) {
  const [value, setValue] = useState(initial);
  return <CollectionField collections={collections} value={value} onChange={setValue} />;
}

describe("CollectionField", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("adds and removes collections while preserving multiple selections", async () => {
    const user = userEvent.setup();
    render(<StatefulField initial={[7]} />);

    expect(screen.getByRole("list", { name: "Selected collections" })).toHaveTextContent("Docker");

    await user.selectOptions(screen.getByLabelText("Collections"), "8");

    expect(screen.getByRole("list", { name: "Selected collections" })).toHaveTextContent(
      "DockerGit",
    );
    await user.click(screen.getByRole("button", { name: "Remove Docker collection" }));

    expect(screen.getByRole("list", { name: "Selected collections" })).not.toHaveTextContent(
      "Docker",
    );
    expect(screen.getByRole("list", { name: "Selected collections" })).toHaveTextContent("Git");
  });

  it("creates a collection inline and selects it", async () => {
    const user = userEvent.setup();
    const utilities: Collection = {
      id: 9,
      name: "Utilities",
      description: "",
      commandCount: 0,
      createdAt: "2026-07-25T00:00:00Z",
      updatedAt: "2026-07-25T00:00:00Z",
    };
    const createCollection = vi
      .spyOn(api, "createCollection")
      .mockResolvedValue([...collections, utilities]);
    render(<StatefulField />);

    await user.selectOptions(screen.getByLabelText("Collections"), "__create_collection__");
    const name = screen.getByLabelText("New collection name");
    expect(name).toHaveFocus();

    await user.type(name, "  Utilities  {Enter}");

    expect(createCollection).toHaveBeenCalledWith({ name: "Utilities", description: "" });
    expect(
      await screen.findByRole("button", { name: "Remove Utilities collection" }),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("New collection name")).not.toBeInTheDocument();
  });

  it("announces backend creation errors and keeps the name available to correct", async () => {
    const user = userEvent.setup();
    vi.spyOn(api, "createCollection").mockRejectedValue({
      kind: "invalid",
      message: 'A collection named "Git" already exists',
    });
    render(<StatefulField />);

    await user.selectOptions(screen.getByLabelText("Collections"), "__create_collection__");
    await user.type(screen.getByLabelText("New collection name"), "Git");
    await user.click(screen.getByRole("button", { name: "Add" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      'A collection named "Git" already exists',
    );
    expect(screen.getByLabelText("New collection name")).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByLabelText("New collection name")).toHaveValue("Git");
  });
});
