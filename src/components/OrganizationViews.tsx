import type { AppView, Collection, Scope, Tag } from "../lib/types";
import { CollectionsBrowser } from "./CollectionsBrowser";
import { TagsBrowser } from "./TagsBrowser";
import { ViewHeader } from "./ViewHeader";
import { Button } from "./ui/Button";

export interface OrganizationViewsProps {
  view: AppView;
  collections: Collection[];
  tags: Tag[];
  scope: Scope;
  onOpenMenu: () => void;
  onOpenLibrary: () => void;
  onManageCollections: () => void;
  onScopeChange: (scope: Scope) => void;
}

export function OrganizationViews({
  view,
  collections,
  tags,
  scope,
  onOpenMenu,
  onOpenLibrary,
  onManageCollections,
  onScopeChange,
}: OrganizationViewsProps) {
  if (view === "collections") {
    return (
      <>
        <ViewHeader
          title="All collections"
          onOpenMenu={onOpenMenu}
          actions={
            <>
              <Button variant="secondary" size="sm" onClick={onManageCollections}>
                Manage collections
              </Button>
              <Button variant="secondary" size="sm" onClick={onOpenLibrary}>
                Back to library
              </Button>
            </>
          }
        />
        <div className="shell__content">
          <CollectionsBrowser
            collections={collections}
            activeCollectionId={scope.type === "collection" ? scope.id : null}
            onSelectCollection={(id) => onScopeChange({ type: "collection", id })}
          />
        </div>
      </>
    );
  }

  if (view === "tags") {
    return (
      <>
        <ViewHeader
          title="All tags"
          onOpenMenu={onOpenMenu}
          actions={
            <Button variant="secondary" size="sm" onClick={onOpenLibrary}>
              Back to library
            </Button>
          }
        />
        <div className="shell__content">
          <TagsBrowser
            tags={tags}
            activeTagName={scope.type === "tag" ? scope.name : null}
            onSelectTag={(name) => onScopeChange({ type: "tag", name })}
          />
        </div>
      </>
    );
  }

  return null;
}
