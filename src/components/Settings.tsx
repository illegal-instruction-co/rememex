import { useState, useEffect } from "react";
import { Settings as SettingsIcon, X, Pin, Rocket, Keyboard, GitBranch, Globe, Brain, Ruler, FilePlus, FileX } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useLocale } from "../i18n";
import "./Settings.css";

interface AppConfig {
    always_on_top: boolean;
    launch_at_startup: boolean;
    hotkey: string;
    use_git_history: boolean;
    embedding_model: string;
    chunk_size: number | null;
    chunk_overlap: number | null;
    extra_extensions: string[];
    excluded_extensions: string[];
}

interface SettingsProps {
    open: boolean;
    onClose: () => void;
}

const localeLabels: Record<string, string> = { en: "English", tr: "Türkçe" };

const modelLabels: Record<string, string> = {
    AllMiniLML6V2: "MiniLM L6 v2 (Fast)",
    MultilingualE5Small: "Multilingual E5 Small",
    MultilingualE5Base: "Multilingual E5 Base",
};

function parseExtensions(raw: string): string[] {
    return raw
        .split(/[,\s]+/)
        .map((s) => s.trim().replace(/^\./, ""))
        .filter((s) => s.length > 0);
}

export function SettingsButton({ onClick }: Readonly<{ onClick: () => void }>) {
    const { t } = useLocale();
    return (
        <button className="sidebar-btn" onClick={onClick} title={t("settings_title")}>
            <SettingsIcon size={14} />
        </button>
    );
}

export default function Settings({ open, onClose }: Readonly<SettingsProps>) {
    const { t, locale, setLocale, availableLocales } = useLocale();
    const [config, setConfig] = useState<AppConfig | null>(null);
    const [hotkeyDraft, setHotkeyDraft] = useState("");
    const [hotkeyDirty, setHotkeyDirty] = useState(false);
    const [extraExtDraft, setExtraExtDraft] = useState("");
    const [excludedExtDraft, setExcludedExtDraft] = useState("");

    useEffect(() => {
        if (open) {
            invoke<AppConfig>("get_config").then((c) => {
                setConfig(c);
                setHotkeyDraft(c.hotkey);
                setHotkeyDirty(false);
                setExtraExtDraft(c.extra_extensions.join(", "));
                setExcludedExtDraft(c.excluded_extensions.join(", "));
            });
        }
    }, [open]);

    async function updateField(updates: Partial<AppConfig>) {
        await invoke("update_config", { updates });
        const updated = await invoke<AppConfig>("get_config");
        setConfig(updated);
        setHotkeyDraft(updated.hotkey);
    }

    if (!open || !config) return null;

    return (
        <div className="settings-overlay" role="none" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
            <div className="settings-panel">
                <div className="settings-header">
                    <h2 className="settings-title">{t("settings_title")}</h2>
                    <button type="button" className="settings-close" onClick={onClose}>
                        <X size={14} />
                    </button>
                </div>

                <div className="settings-body">
                    <div className="settings-group">
                        <div className="settings-row">
                            <div className="settings-row-info">
                                <Pin size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_always_on_top")}</span>
                                    <span className="settings-row-desc">{t("settings_always_on_top_desc")}</span>
                                </div>
                            </div>
                            <label className="toggle" aria-label={t("settings_always_on_top")}>
                                <input
                                    type="checkbox"
                                    checked={config.always_on_top}
                                    onChange={(e) => updateField({ always_on_top: e.target.checked })}
                                />
                                <span className="toggle-slider" />
                            </label>
                        </div>

                        <div className="settings-row">
                            <div className="settings-row-info">
                                <Rocket size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_launch_startup")}</span>
                                    <span className="settings-row-desc">{t("settings_launch_startup_desc")}</span>
                                </div>
                            </div>
                            <label className="toggle" aria-label={t("settings_launch_startup")}>
                                <input
                                    type="checkbox"
                                    checked={config.launch_at_startup}
                                    onChange={(e) => updateField({ launch_at_startup: e.target.checked })}
                                />
                                <span className="toggle-slider" />
                            </label>
                        </div>

                        <div className="settings-row hotkey-row">
                            <div className="settings-row-info">
                                <Keyboard size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_hotkey")}</span>
                                    <span className="settings-row-desc">{t("settings_hotkey_desc")}</span>
                                </div>
                            </div>
                            <div className="hotkey-input-wrapper">
                                <input
                                    type="text"
                                    className="hotkey-input"
                                    value={hotkeyDraft}
                                    onChange={(e) => {
                                        setHotkeyDraft(e.target.value);
                                        setHotkeyDirty(e.target.value !== config.hotkey);
                                    }}
                                    onKeyDown={(e) => {
                                        if (e.key === "Enter" && hotkeyDirty) {
                                            e.preventDefault();
                                            updateField({ hotkey: hotkeyDraft }).then(() => setHotkeyDirty(false));
                                        }
                                    }}
                                    spellCheck={false}
                                />
                                {hotkeyDirty && (
                                    <button
                                        className="hotkey-save"
                                        onClick={() => updateField({ hotkey: hotkeyDraft }).then(() => setHotkeyDirty(false))}
                                    >
                                        ↵
                                    </button>
                                )}
                            </div>
                            {hotkeyDirty && (
                                <span className="settings-row-note">{t("settings_restart_required")}</span>
                            )}
                        </div>

                        <div className="settings-row">
                            <div className="settings-row-info">
                                <Globe size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_language")}</span>
                                    <span className="settings-row-desc">{t("settings_language_desc")}</span>
                                </div>
                            </div>
                            <select
                                className="settings-select"
                                value={locale}
                                aria-label={t("settings_language")}
                                onChange={(e) => setLocale(e.target.value)}
                            >
                                {availableLocales.map((loc) => (
                                    <option key={loc} value={loc}>
                                        {localeLabels[loc] ?? loc}
                                    </option>
                                ))}
                            </select>
                        </div>
                    </div>

                    <div className="settings-group">
                        <div className="settings-section-title">{t("settings_section_indexing")}</div>

                        <div className="settings-row">
                            <div className="settings-row-info">
                                <GitBranch size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_git_history")}</span>
                                    <span className="settings-row-desc">{t("settings_git_history_desc")}</span>
                                </div>
                            </div>
                            <label className="toggle" aria-label={t("settings_git_history")}>
                                <input
                                    type="checkbox"
                                    checked={config.use_git_history}
                                    onChange={(e) => updateField({ use_git_history: e.target.checked })}
                                />
                                <span className="toggle-slider" />
                            </label>
                        </div>

                        <div className="settings-row">
                            <div className="settings-row-info">
                                <Brain size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_embedding_model")}</span>
                                    <span className="settings-row-desc">{t("settings_embedding_model_desc")}</span>
                                </div>
                            </div>
                            <select
                                className="settings-select"
                                value={config.embedding_model}
                                aria-label={t("settings_embedding_model")}
                                onChange={(e) => updateField({ embedding_model: e.target.value })}
                            >
                                {Object.entries(modelLabels).map(([key, label]) => (
                                    <option key={key} value={key}>
                                        {label}
                                    </option>
                                ))}
                            </select>
                        </div>
                        {config.embedding_model !== "MultilingualE5Base" && (
                            <span className="settings-row-note">{t("settings_restart_reindex")}</span>
                        )}

                        <div className="settings-row">
                            <div className="settings-row-info">
                                <Ruler size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_chunk_size")}</span>
                                    <span className="settings-row-desc">{t("settings_chunk_desc")}</span>
                                </div>
                            </div>
                            <div className="settings-number-group">
                                <input
                                    type="number"
                                    className="settings-number-input"
                                    value={config.chunk_size ?? ""}
                                    placeholder="512"
                                    aria-label={t("settings_chunk_size")}
                                    min={64}
                                    max={4096}
                                    onChange={(e) => {
                                        const v = e.target.value ? Number.parseInt(e.target.value, 10) : null;
                                        updateField({ chunk_size: v });
                                    }}
                                />
                                <span className="settings-number-label">{t("settings_chunk_overlap")}</span>
                                <input
                                    type="number"
                                    className="settings-number-input"
                                    value={config.chunk_overlap ?? ""}
                                    placeholder="64"
                                    aria-label={t("settings_chunk_overlap")}
                                    min={0}
                                    max={512}
                                    onChange={(e) => {
                                        const v = e.target.value ? Number.parseInt(e.target.value, 10) : null;
                                        updateField({ chunk_overlap: v });
                                    }}
                                />
                            </div>
                        </div>

                        <div className="settings-row">
                            <div className="settings-row-info">
                                <FilePlus size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_extra_ext")}</span>
                                    <span className="settings-row-desc">{t("settings_extra_ext_desc")}</span>
                                </div>
                            </div>
                            <input
                                type="text"
                                className="settings-ext-input"
                                value={extraExtDraft}
                                placeholder=".xyz, .abc"
                                aria-label={t("settings_extra_ext")}
                                spellCheck={false}
                                onChange={(e) => setExtraExtDraft(e.target.value)}
                                onBlur={() => updateField({ extra_extensions: parseExtensions(extraExtDraft) })}
                                onKeyDown={(e) => {
                                    if (e.key === "Enter") {
                                        updateField({ extra_extensions: parseExtensions(extraExtDraft) });
                                    }
                                }}
                            />
                        </div>

                        <div className="settings-row">
                            <div className="settings-row-info">
                                <FileX size={14} className="settings-row-icon" />
                                <div>
                                    <span className="settings-row-label">{t("settings_excluded_ext")}</span>
                                    <span className="settings-row-desc">{t("settings_excluded_ext_desc")}</span>
                                </div>
                            </div>
                            <input
                                type="text"
                                className="settings-ext-input"
                                value={excludedExtDraft}
                                placeholder=".log, .tmp"
                                aria-label={t("settings_excluded_ext")}
                                spellCheck={false}
                                onChange={(e) => setExcludedExtDraft(e.target.value)}
                                onBlur={() => updateField({ excluded_extensions: parseExtensions(excludedExtDraft) })}
                                onKeyDown={(e) => {
                                    if (e.key === "Enter") {
                                        updateField({ excluded_extensions: parseExtensions(excludedExtDraft) });
                                    }
                                }}
                            />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
