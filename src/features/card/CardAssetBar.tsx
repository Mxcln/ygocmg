import type { CardAssetState, PrimaryType, SpellSubtype } from "../../shared/contracts/card";

interface CardAssetBarProps {
  assetState: CardAssetState;
  primaryType: PrimaryType;
  spellSubtype: SpellSubtype | null;
}

export function CardAssetBar({ assetState, primaryType, spellSubtype }: CardAssetBarProps) {
  const isFieldSpell = primaryType === "spell" && spellSubtype === "field";

  return (
    <div className="card-asset-bar">
      <div className="card-pic-preview">No Image</div>

      <div className="asset-btn-group">
        <span className="asset-btn-group-label">Image</span>
        <button type="button" className="asset-seg-btn" disabled>Import</button>
        {assetState.has_image && (
          <button type="button" className="asset-seg-btn danger" disabled>Delete</button>
        )}
      </div>

      <div className="asset-btn-group">
        <span className="asset-btn-group-label">Script</span>
        {assetState.has_script ? (
          <>
            <button type="button" className="asset-seg-btn" disabled>Import</button>
            <button type="button" className="asset-seg-btn" disabled>Edit</button>
            <button type="button" className="asset-seg-btn danger" disabled>Delete</button>
          </>
        ) : (
          <>
            <button type="button" className="asset-seg-btn" disabled>Create</button>
            <button type="button" className="asset-seg-btn" disabled>Import</button>
          </>
        )}
      </div>

      {isFieldSpell && (
        <div className="asset-btn-group">
          <span className="asset-btn-group-label">Field</span>
          <button type="button" className="asset-seg-btn" disabled>Import</button>
          {assetState.has_field_image && (
            <button type="button" className="asset-seg-btn danger" disabled>Delete</button>
          )}
        </div>
      )}
    </div>
  );
}
