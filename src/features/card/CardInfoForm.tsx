import type {
  CardEntity,
  PrimaryType,
  Ot,
  MonsterFlag,
  Race,
  Attribute,
  SpellSubtype,
  TrapSubtype,
  LinkMarker,
} from "../../shared/contracts/card";

interface CardInfoFormProps {
  draft: CardEntity;
  onChange: (patch: Partial<CardEntity>) => void;
}

const ALL_OT: Ot[] = ["ocg", "tcg", "custom"];

const ALL_MONSTER_FLAGS: MonsterFlag[] = [
  "normal", "effect", "fusion", "ritual", "synchro", "xyz",
  "pendulum", "link", "tuner", "token", "gemini", "spirit",
  "union", "flip", "toon",
];

const ALL_RACES: Race[] = [
  "warrior", "spellcaster", "fairy", "fiend", "zombie", "machine",
  "aqua", "pyro", "rock", "winged_beast", "plant", "insect",
  "thunder", "dragon", "beast", "beast_warrior", "dinosaur",
  "fish", "sea_serpent", "reptile", "psychic", "divine_beast",
  "creator_god", "wyrm", "cyberse", "illusion",
];

const ALL_ATTRIBUTES: Attribute[] = [
  "light", "dark", "earth", "water", "fire", "wind", "divine",
];

const ALL_SPELL_SUBTYPES: SpellSubtype[] = [
  "normal", "continuous", "quick_play", "ritual", "field", "equip",
];

const ALL_TRAP_SUBTYPES: TrapSubtype[] = [
  "normal", "continuous", "counter",
];

const LINK_MARKER_POSITIONS: (LinkMarker | null)[][] = [
  ["top_left", "top", "top_right"],
  ["left", null, "right"],
  ["bottom_left", "bottom", "bottom_right"],
];

const LINK_MARKER_ARROWS: Record<LinkMarker, string> = {
  top_left: "\u2196", top: "\u2191", top_right: "\u2197",
  left: "\u2190", right: "\u2192",
  bottom_left: "\u2199", bottom: "\u2193", bottom_right: "\u2198",
};

function displayLabel(value: string): string {
  return value.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

export function CardInfoForm({ draft, onChange }: CardInfoFormProps) {
  const isMonster = draft.primary_type === "monster";
  const isSpell = draft.primary_type === "spell";
  const isTrap = draft.primary_type === "trap";
  const flags = draft.monster_flags ?? [];
  const isLink = isMonster && flags.includes("link");
  const isPendulum = isMonster && flags.includes("pendulum");

  function handlePrimaryTypeChange(pt: PrimaryType) {
    if (pt === draft.primary_type) return;
    const patch: Partial<CardEntity> = { primary_type: pt };
    if (pt !== "monster") {
      patch.monster_flags = null;
      patch.atk = null;
      patch.def = null;
      patch.race = null;
      patch.attribute = null;
      patch.level = null;
      patch.pendulum = null;
      patch.link = null;
    }
    if (pt !== "spell") patch.spell_subtype = null;
    if (pt !== "trap") patch.trap_subtype = null;
    onChange(patch);
  }

  function toggleMonsterFlag(flag: MonsterFlag) {
    const current = draft.monster_flags ?? [];
    const next = current.includes(flag)
      ? current.filter((f) => f !== flag)
      : [...current, flag];

    const patch: Partial<CardEntity> = { monster_flags: next };

    if (flag === "link" && !current.includes("link")) {
      patch.def = null;
      patch.level = null;
      patch.link = { markers: [] };
    }
    if (flag === "link" && current.includes("link")) {
      patch.link = null;
    }
    if (flag === "pendulum" && !current.includes("pendulum")) {
      patch.pendulum = { left_scale: 0, right_scale: 0 };
    }
    if (flag === "pendulum" && current.includes("pendulum")) {
      patch.pendulum = null;
    }
    onChange(patch);
  }

  function toggleLinkMarker(marker: LinkMarker) {
    const current = draft.link?.markers ?? [];
    const next = current.includes(marker)
      ? current.filter((m) => m !== marker)
      : [...current, marker];
    onChange({ link: { markers: next } });
  }

  function handleNumberInput(
    field: "code" | "alias" | "setcode" | "category" | "atk" | "def" | "level",
    value: string,
  ) {
    if (value === "" || value === "-") {
      onChange({ [field]: 0 });
      return;
    }
    if (value === "?") {
      if (field === "atk" || field === "def") {
        onChange({ [field]: -2 });
      }
      return;
    }
    const parsed = Number.parseInt(value, 10);
    if (Number.isFinite(parsed)) {
      onChange({ [field]: parsed });
    }
  }

  function formatStatValue(val: number | null): string {
    if (val === null) return "";
    if (val === -2) return "?";
    return String(val);
  }

  return (
    <div className="card-info-grid">
      {/* Basic fields */}
      <div className="card-info-field">
        <label className="card-info-label">Code</label>
        <input
          className="card-info-input"
          type="text"
          inputMode="numeric"
          value={draft.code}
          onChange={(e) => handleNumberInput("code", e.target.value)}
        />
      </div>
      <div className="card-info-field">
        <label className="card-info-label">Alias</label>
        <input
          className="card-info-input"
          type="text"
          inputMode="numeric"
          value={draft.alias}
          onChange={(e) => handleNumberInput("alias", e.target.value)}
        />
      </div>

      <div className="card-info-field">
        <label className="card-info-label">Setcode</label>
        <input
          className="card-info-input"
          type="text"
          value={`0x${draft.setcode.toString(16).toUpperCase()}`}
          onChange={(e) => {
            const raw = e.target.value.replace(/^0x/i, "");
            const parsed = Number.parseInt(raw, 16);
            if (Number.isFinite(parsed)) onChange({ setcode: parsed });
            else if (raw === "") onChange({ setcode: 0 });
          }}
        />
      </div>
      <div className="card-info-field">
        <label className="card-info-label">OT</label>
        <select
          className="card-info-select"
          value={draft.ot}
          onChange={(e) => onChange({ ot: e.target.value as Ot })}
        >
          {ALL_OT.map((o) => (
            <option key={o} value={o}>{displayLabel(o)}</option>
          ))}
        </select>
      </div>

      <div className="card-info-field">
        <label className="card-info-label">Category</label>
        <input
          className="card-info-input"
          type="text"
          value={`0x${draft.category.toString(16).toUpperCase()}`}
          onChange={(e) => {
            const raw = e.target.value.replace(/^0x/i, "");
            const parsed = Number.parseInt(raw, 16);
            if (Number.isFinite(parsed)) onChange({ category: parsed });
            else if (raw === "") onChange({ category: 0 });
          }}
        />
      </div>
      <div className="card-info-field">
        <label className="card-info-label">Primary Type</label>
        <div className="card-type-radio-group">
          {(["monster", "spell", "trap"] as PrimaryType[]).map((pt) => (
            <button
              key={pt}
              type="button"
              className={`card-type-radio ${draft.primary_type === pt ? "active" : ""}`}
              onClick={() => handlePrimaryTypeChange(pt)}
            >
              {displayLabel(pt)}
            </button>
          ))}
        </div>
      </div>

      {/* Monster-specific fields */}
      {isMonster && (
        <div className="card-info-section">
          <h4 className="card-info-section-title">Monster</h4>
          <div className="card-info-grid">
            <div className="card-info-field full-width">
              <label className="card-info-label">Monster Flags</label>
              <div className="monster-flags-group">
                {ALL_MONSTER_FLAGS.map((flag) => (
                  <button
                    key={flag}
                    type="button"
                    className={`monster-flag-chip ${flags.includes(flag) ? "selected" : ""}`}
                    onClick={() => toggleMonsterFlag(flag)}
                  >
                    {displayLabel(flag)}
                  </button>
                ))}
              </div>
            </div>

            <div className="card-info-field">
              <label className="card-info-label">Attribute</label>
              <select
                className="card-info-select"
                value={draft.attribute ?? ""}
                onChange={(e) =>
                  onChange({ attribute: (e.target.value || null) as Attribute | null })
                }
              >
                <option value="">—</option>
                {ALL_ATTRIBUTES.map((a) => (
                  <option key={a} value={a}>{displayLabel(a)}</option>
                ))}
              </select>
            </div>
            <div className="card-info-field">
              <label className="card-info-label">Race</label>
              <select
                className="card-info-select"
                value={draft.race ?? ""}
                onChange={(e) =>
                  onChange({ race: (e.target.value || null) as Race | null })
                }
              >
                <option value="">—</option>
                {ALL_RACES.map((r) => (
                  <option key={r} value={r}>{displayLabel(r)}</option>
                ))}
              </select>
            </div>

            <div className="card-info-field">
              <label className="card-info-label">ATK</label>
              <input
                className="card-info-input"
                type="text"
                value={formatStatValue(draft.atk)}
                onChange={(e) => handleNumberInput("atk", e.target.value)}
                placeholder="e.g. 2500 or ?"
              />
            </div>
            {!isLink && (
              <div className="card-info-field">
                <label className="card-info-label">DEF</label>
                <input
                  className="card-info-input"
                  type="text"
                  value={formatStatValue(draft.def)}
                  onChange={(e) => handleNumberInput("def", e.target.value)}
                  placeholder="e.g. 2000 or ?"
                />
              </div>
            )}
            {!isLink && (
              <div className="card-info-field">
                <label className="card-info-label">
                  {flags.includes("xyz") ? "Rank" : "Level"}
                </label>
                <input
                  className="card-info-input"
                  type="text"
                  inputMode="numeric"
                  value={draft.level ?? ""}
                  onChange={(e) => handleNumberInput("level", e.target.value)}
                />
              </div>
            )}

            {isPendulum && (
              <>
                <div className="card-info-field">
                  <label className="card-info-label">Left Scale</label>
                  <input
                    className="card-info-input"
                    type="text"
                    inputMode="numeric"
                    value={draft.pendulum?.left_scale ?? 0}
                    onChange={(e) => {
                      const v = Number.parseInt(e.target.value, 10);
                      onChange({
                        pendulum: {
                          left_scale: Number.isFinite(v) ? v : 0,
                          right_scale: draft.pendulum?.right_scale ?? 0,
                        },
                      });
                    }}
                  />
                </div>
                <div className="card-info-field">
                  <label className="card-info-label">Right Scale</label>
                  <input
                    className="card-info-input"
                    type="text"
                    inputMode="numeric"
                    value={draft.pendulum?.right_scale ?? 0}
                    onChange={(e) => {
                      const v = Number.parseInt(e.target.value, 10);
                      onChange({
                        pendulum: {
                          left_scale: draft.pendulum?.left_scale ?? 0,
                          right_scale: Number.isFinite(v) ? v : 0,
                        },
                      });
                    }}
                  />
                </div>
              </>
            )}

            {isLink && (
              <div className="card-info-field full-width">
                <label className="card-info-label">Link Markers</label>
                <div className="link-marker-grid">
                  {LINK_MARKER_POSITIONS.flat().map((marker, i) => {
                    if (marker === null) {
                      return <div key={i} className="link-marker-cell center" />;
                    }
                    const selected = draft.link?.markers.includes(marker) ?? false;
                    return (
                      <button
                        key={marker}
                        type="button"
                        className={`link-marker-cell ${selected ? "selected" : ""}`}
                        onClick={() => toggleLinkMarker(marker)}
                        title={displayLabel(marker)}
                      >
                        {LINK_MARKER_ARROWS[marker]}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Spell-specific fields */}
      {isSpell && (
        <div className="card-info-section">
          <h4 className="card-info-section-title">Spell</h4>
          <div className="card-info-grid">
            <div className="card-info-field">
              <label className="card-info-label">Spell Subtype</label>
              <select
                className="card-info-select"
                value={draft.spell_subtype ?? ""}
                onChange={(e) =>
                  onChange({ spell_subtype: (e.target.value || null) as SpellSubtype | null })
                }
              >
                <option value="">—</option>
                {ALL_SPELL_SUBTYPES.map((s) => (
                  <option key={s} value={s}>{displayLabel(s)}</option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}

      {/* Trap-specific fields */}
      {isTrap && (
        <div className="card-info-section">
          <h4 className="card-info-section-title">Trap</h4>
          <div className="card-info-grid">
            <div className="card-info-field">
              <label className="card-info-label">Trap Subtype</label>
              <select
                className="card-info-select"
                value={draft.trap_subtype ?? ""}
                onChange={(e) =>
                  onChange({ trap_subtype: (e.target.value || null) as TrapSubtype | null })
                }
              >
                <option value="">—</option>
                {ALL_TRAP_SUBTYPES.map((t) => (
                  <option key={t} value={t}>{displayLabel(t)}</option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
