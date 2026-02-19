import { Search, Brain, Shuffle, Sparkles } from "lucide-react";
import { useLocale } from "../../i18n";
import { SettingsRow, SettingsToggle } from "./SettingsRow";
import "./SearchSettings.css";

interface AppConfig {
    use_reranker: boolean;
    hyde_enabled: boolean;
    hyde_endpoint: string;
    hyde_model: string;
    hyde_api_key: string;
    query_router_enabled: boolean;
    mmr_enabled: boolean;
    mmr_lambda: number;
}

interface Props {
    config: AppConfig;
    updateField: (updates: Record<string, unknown>) => Promise<void>;
}

export default function SearchSettings({ config, updateField }: Readonly<Props>) {
    const { t } = useLocale();

    return (
        <>
            <SettingsRow
                icon={<Search size={14} />}
                label={t("settings_query_router")}
                desc={t("settings_query_router_desc")}
                control={
                    <SettingsToggle
                        label={t("settings_query_router")}
                        checked={config.query_router_enabled}
                        onChange={(v) => updateField({ query_router_enabled: v })}
                    />
                }
            />

            <SettingsRow
                icon={<Shuffle size={14} />}
                label={t("settings_mmr")}
                desc={t("settings_mmr_desc")}
                control={
                    <SettingsToggle
                        label={t("settings_mmr")}
                        checked={config.mmr_enabled}
                        onChange={(v) => updateField({ mmr_enabled: v })}
                    />
                }
            />

            {config.mmr_enabled && (
                <SettingsRow
                    icon={<Shuffle size={14} />}
                    label={t("settings_mmr_lambda")}
                    desc={t("settings_mmr_lambda_desc")}
                    control={
                        <input
                            type="range"
                            className="settings-range"
                            min={0}
                            max={100}
                            value={Math.round(config.mmr_lambda * 100)}
                            onChange={(e) =>
                                updateField({ mmr_lambda: Number.parseInt(e.target.value) / 100 })
                            }
                            aria-label={t("settings_mmr_lambda")}
                            title={`${Math.round(config.mmr_lambda * 100)}%`}
                        />
                    }
                />
            )}

            <SettingsRow
                icon={<Sparkles size={14} />}
                label={t("settings_hyde")}
                desc={t("settings_hyde_desc")}
                control={
                    <SettingsToggle
                        label={t("settings_hyde")}
                        checked={config.hyde_enabled}
                        onChange={(v) => updateField({ hyde_enabled: v })}
                    />
                }
            />

            {config.hyde_enabled && (
                <>
                    <SettingsRow
                        icon={<Brain size={14} />}
                        label={t("settings_hyde_endpoint")}
                        desc={t("settings_hyde_endpoint_desc")}
                        control={
                            <input
                                type="text"
                                className="settings-input"
                                value={config.hyde_endpoint}
                                placeholder="http://localhost:11434/v1/chat/completions"
                                onChange={(e) => updateField({ hyde_endpoint: e.target.value })}
                                spellCheck={false}
                            />
                        }
                    />
                    <SettingsRow
                        icon={<Brain size={14} />}
                        label={t("settings_hyde_model")}
                        desc={t("settings_hyde_model_desc")}
                        control={
                            <input
                                type="text"
                                className="settings-input"
                                value={config.hyde_model}
                                placeholder="llama3.2"
                                onChange={(e) => updateField({ hyde_model: e.target.value })}
                                spellCheck={false}
                            />
                        }
                    />
                    <SettingsRow
                        icon={<Brain size={14} />}
                        label={t("settings_hyde_api_key")}
                        desc={t("settings_hyde_api_key_desc")}
                        control={
                            <input
                                type="password"
                                className="settings-input"
                                value={config.hyde_api_key}
                                placeholder="sk-..."
                                onChange={(e) => updateField({ hyde_api_key: e.target.value })}
                                spellCheck={false}
                            />
                        }
                    />
                </>
            )}
        </>
    );
}
