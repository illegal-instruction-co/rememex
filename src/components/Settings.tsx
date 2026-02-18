import { useState, useEffect } from "react";
import { Settings as SettingsIcon, X, Pin, Rocket, Keyboard } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useLocale } from "../i18n";
import "./Settings.css";

interface AppConfig {
    always_on_top: boolean;
    launch_at_startup: boolean;
    hotkey: string;
}

interface SettingsProps {
    open: boolean;
    onClose: () => void;
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
    const { t } = useLocale();
    const [config, setConfig] = useState<AppConfig | null>(null);
    const [hotkeyDraft, setHotkeyDraft] = useState("");
    const [hotkeyDirty, setHotkeyDirty] = useState(false);

    useEffect(() => {
        if (open) {
            invoke<AppConfig>("get_config").then((c) => {
                setConfig(c);
                setHotkeyDraft(c.hotkey);
                setHotkeyDirty(false);
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
                    </div>
                </div>
            </div>
        </div>
    );
}
