import {
    Box, Plus, Trash2, FolderOpen, Folder, RefreshCw,
    PanelLeftClose, PanelLeftOpen, Globe, MessageSquarePlus, ChevronDown, ChevronRight, Search,
} from "lucide-react";
import { SettingsButton } from "./Settings";
import type { ContainerItem } from "../types";
import { useLocale } from "../i18n";
import { useState, useMemo } from "react";

const localeLabels: Record<string, string> = {
    en: "English",
    tr: "Türkçe",
};

interface Annotation {
    id: string;
    path: string;
    note: string;
    source: string;
    created_at: number;
}

interface SidebarProps {
    containers: ContainerItem[];
    activeContainer: string;
    isIndexing: boolean;
    sidebarOpen: boolean;
    annotations: Annotation[];
    onToggleSidebar: () => void;
    onSwitchContainer: (name: string) => void;
    onCreateContainer: () => void;
    onDeleteContainer: () => void;
    onReindexAll: () => void;
    onOpenSettings: () => void;
    onDeleteAnnotation: (id: string) => void;
    onSelectAnnotation: (id: string) => void;
}

export default function Sidebar({
    containers, activeContainer, isIndexing, sidebarOpen, annotations,
    onToggleSidebar, onSwitchContainer, onCreateContainer,
    onDeleteContainer, onReindexAll, onOpenSettings, onDeleteAnnotation: _onDeleteAnnotation, onSelectAnnotation,
}: Readonly<SidebarProps>) {
    const { t, locale, setLocale, availableLocales } = useLocale();
    const [annotationsOpen, setAnnotationsOpen] = useState(false);
    const [annotationFilter, setAnnotationFilter] = useState("");
    const [annotationLimit, setAnnotationLimit] = useState(20);
    const [sourceFilter, setSourceFilter] = useState<'all' | 'user' | 'agent'>('all');

    const filteredAnnotations = useMemo(() => {
        let list = annotations;
        if (sourceFilter !== 'all') {
            list = list.filter(a => a.source === sourceFilter);
        }
        if (annotationFilter.trim()) {
            const q = annotationFilter.toLowerCase();
            list = list.filter(a =>
                a.path.toLowerCase().includes(q) || a.note.toLowerCase().includes(q)
            );
        }
        return list;
    }, [annotations, annotationFilter, sourceFilter]);

    function cycleLocale() {
        const idx = availableLocales.indexOf(locale);
        setLocale(availableLocales[(idx + 1) % availableLocales.length]);
    }

    return (
        <div className={`sidebar ${sidebarOpen ? '' : 'collapsed'}`}>
            <div className="sidebar-header">
                <button className="sidebar-btn" onClick={onToggleSidebar} title={sidebarOpen ? t('sidebar_collapse') : t('sidebar_expand')}>
                    {sidebarOpen ? <PanelLeftClose size={14} /> : <PanelLeftOpen size={14} />}
                </button>
                {sidebarOpen && (
                    <>
                        <span className="sidebar-title sidebar-title-flex">{t('sidebar_title')}</span>
                        <SettingsButton onClick={onOpenSettings} />
                        <button className="sidebar-btn" onClick={onCreateContainer} title={t('sidebar_create')}>
                            <Plus size={14} />
                        </button>
                    </>
                )}
            </div>
            {sidebarOpen && (
                <>
                    <div className="container-list">
                        {containers.map(c => (
                            <div key={c.name} className="container-item-wrapper">
                                <button
                                    type="button"
                                    className={`container-item container-item-full ${activeContainer === c.name ? 'active' : ''} ${isIndexing ? 'disabled' : ''}`}
                                    onClick={() => !isIndexing && onSwitchContainer(c.name)}
                                    disabled={isIndexing}
                                >
                                    <Box size={14} className="icon" />
                                    <div className="container-item-content">
                                        <span className="container-item-name">{c.name}</span>
                                        {c.description && (
                                            <span className="container-item-desc">{c.description}</span>
                                        )}
                                        {c.provider_label && (
                                            <span className="container-item-desc" style={{ opacity: 0.3, fontSize: '9px' }}>{c.provider_label}</span>
                                        )}
                                    </div>
                                </button>
                                {activeContainer === c.name && (
                                    <div className="indexed-paths-section">
                                        <div className="indexed-paths-header">
                                            <Folder size={10} className="indexed-paths-icon" />
                                            <span>{t('sidebar_indexed_folders')}</span>
                                        </div>
                                        {c.indexed_paths.length > 0 ? (
                                            <>
                                                <div className="indexed-paths">
                                                    {c.indexed_paths.map(p => (
                                                        <div key={p} className="indexed-path-item" title={p}>
                                                            <FolderOpen size={10} className="indexed-path-icon" />
                                                            <span className="indexed-path-text">{p.split(/[\\/]/).slice(-2).join('/')}</span>
                                                        </div>
                                                    ))}
                                                </div>
                                                <button
                                                    className="reindex-btn"
                                                    onClick={onReindexAll}
                                                    disabled={isIndexing}
                                                    title={t('sidebar_rebuild_tooltip')}
                                                >
                                                    <RefreshCw size={10} className={isIndexing ? 'reindex-spin' : ''} />
                                                    <span>{t('sidebar_rebuild')}</span>
                                                </button>
                                            </>
                                        ) : (
                                            <div className="indexed-paths-empty">
                                                {t('sidebar_no_folders')}
                                            </div>
                                        )}
                                    </div>
                                )}
                            </div>
                        ))}
                    </div>
                    {activeContainer !== "Default" && (
                        <button className="sidebar-delete-btn" onClick={onDeleteContainer}>
                            <Trash2 size={12} /> {t('sidebar_delete')}
                        </button>
                    )}
                    <div className="annotations-section">
                        <button
                            type="button"
                            className="annotations-toggle"
                            onClick={() => setAnnotationsOpen(!annotationsOpen)}
                        >
                            {annotationsOpen ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
                            <MessageSquarePlus size={10} />
                            <span>{t('sidebar_annotations')}</span>
                            {annotations.length > 0 && (
                                <span className="annotations-count">{annotations.length > 99 ? '99+' : annotations.length}</span>
                            )}
                        </button>
                        {annotationsOpen && (
                            <div className="annotations-panel">
                                <div className="annotations-source-tabs">
                                    {(['all', 'user', 'agent'] as const).map(tab => (
                                        <button
                                            key={tab}
                                            type="button"
                                            className={`annotations-tab ${sourceFilter === tab ? 'active' : ''}`}
                                            onClick={() => { setSourceFilter(tab); setAnnotationLimit(20); }}
                                        >
                                            {t({ all: 'annotation_source_all', user: 'annotation_source_user', agent: 'annotation_source_agent' }[tab] as Parameters<typeof t>[0])}
                                        </button>
                                    ))}
                                </div>
                                {annotations.length > 3 && (
                                    <div className="annotations-search">
                                        <Search size={10} className="annotations-search-icon" />
                                        <input
                                            type="text"
                                            className="annotations-search-input"
                                            placeholder={t('annotation_filter')}
                                            value={annotationFilter}
                                            onChange={(e) => { setAnnotationFilter(e.target.value); setAnnotationLimit(20); }}
                                        />
                                    </div>
                                )}
                                <div className="annotations-list">
                                    {filteredAnnotations.length === 0 ? (
                                        <div className="annotations-empty">
                                            {annotations.length === 0 ? t('sidebar_no_annotations') : t('annotation_no_match')}
                                        </div>
                                    ) : (
                                        <>
                                            {filteredAnnotations.slice(0, annotationLimit).map(a => (
                                                <button
                                                    key={a.id}
                                                    type="button"
                                                    className="annotation-item"
                                                    onClick={() => onSelectAnnotation(a.id)}
                                                >
                                                    <div className="annotation-item-content">
                                                        <div className="annotation-item-header">
                                                            <span className="annotation-item-path" title={a.path}>
                                                                {a.path.split(/[\\/]/).pop()}
                                                            </span>
                                                            <span className={`annotation-source-badge ${a.source}`}>
                                                                {a.source === 'agent' ? '🤖' : '👤'}
                                                            </span>
                                                        </div>
                                                        <span className="annotation-item-note">{a.note}</span>
                                                    </div>
                                                </button>
                                            ))}
                                            {filteredAnnotations.length > annotationLimit && (
                                                <button
                                                    type="button"
                                                    className="annotations-show-more"
                                                    onClick={() => setAnnotationLimit(prev => prev + 20)}
                                                >
                                                    {t('annotation_show_more', { count: String(filteredAnnotations.length - annotationLimit) })}
                                                </button>
                                            )}
                                        </>
                                    )}
                                </div>
                            </div>
                        )}
                    </div>
                    <button className="locale-switcher" onClick={cycleLocale} title={localeLabels[locale] ?? locale}>
                        <Globe size={12} />
                        <span>{locale.toUpperCase()}</span>
                    </button>
                </>
            )}
        </div>
    );
}
