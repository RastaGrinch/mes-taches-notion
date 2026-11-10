import {
  CalendarDays,
  Check,
  ChevronRight,
  CircleAlert,
  CloudOff,
  Inbox,
  ListTodo,
  Menu,
  Palette,
  RefreshCw,
  Search,
  Settings as SettingsIcon,
  Star,
  Sun,
  Trash2,
  X,
} from "lucide-react";
import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { backend } from "./api";
import { loadThemePreferences, saveThemePreferences, themes, themeStyle, type AppTheme } from "./themes";
import type { AppStatus, Priority, Settings, Task, ViewKey } from "./types";

const initialSettings: Settings = {
  dataSourceId: "",
  defaultStatus: "A faire",
  defaultPriority: "P3 - A faire",
  defaultAssignee: "",
  defaultSource: "",
  syncIntervalMinutes: 5,
  sourceOptions: [],
  assigneeOptions: [],
};

const navigation: Array<{ key: ViewKey; label: string; icon: typeof Sun }> = [
  { key: "today", label: "Ma journée", icon: Sun },
  { key: "todo", label: "À faire", icon: ListTodo },
  { key: "important", label: "Importantes", icon: Star },
  { key: "later", label: "Plus tard", icon: CalendarDays },
  { key: "completed", label: "Terminées", icon: Check },
];

function isToday(date: string | null) {
  if (!date) return false;
  const today = new Date();
  const value = new Date(`${date}T00:00:00`);
  return value.toDateString() === today.toDateString();
}

function isOverdue(date: string | null) {
  if (!date) return false;
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  return new Date(`${date}T00:00:00`).getTime() < today.getTime();
}

function formatDue(date: string | null) {
  if (!date) return null;
  if (isToday(date)) return "Aujourd’hui";
  if (isOverdue(date)) return `En retard · ${new Intl.DateTimeFormat("fr-FR", { day: "numeric", month: "short" }).format(new Date(`${date}T00:00:00`))}`;
  return new Intl.DateTimeFormat("fr-FR", { weekday: "short", day: "numeric", month: "short" }).format(new Date(`${date}T00:00:00`));
}

export default function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [activeView, setActiveView] = useState<ViewKey>("todo");
  const [selected, setSelected] = useState<Task | null>(null);
  const [query, setQuery] = useState("");
  const [newTitle, setNewTitle] = useState("");
  const [syncing, setSyncing] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [themeOpen, setThemeOpen] = useState(false);
  const [deleteCandidate, setDeleteCandidate] = useState<Task | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [themePreferences, setThemePreferences] = useState(loadThemePreferences);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [nextStatus, nextTasks] = await Promise.all([
        backend.status(),
        backend.listTasks(),
      ]);
      setStatus(nextStatus);
      setTasks(nextTasks);
      setError(null);
    } catch (reason) {
      if (location.protocol.startsWith("http") && !("__TAURI_INTERNALS__" in window)) {
        setStatus({
          configured: false,
          tokenStored: false,
          settings: initialSettings,
          pendingCount: 0,
        });
        setTasks([]);
        setError(null);
        return;
      }
      setError(String(reason));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!status?.configured) return;
    const interval = window.setInterval(() => void handleSync(), Math.max(1, status.settings.syncIntervalMinutes) * 60_000);
    return () => window.clearInterval(interval);
  }, [status?.configured, status?.settings.syncIntervalMinutes]);

  async function handleAdd(event: FormEvent) {
    event.preventDefault();
    const title = newTitle.trim();
    if (!title) return;
    setNewTitle("");
    try {
      const task = await backend.createTask(title);
      setTasks((current) => [task, ...current]);
      setStatus((current) => current ? { ...current, pendingCount: current.pendingCount + 1 } : current);
      if (status?.configured) void handleSync();
    } catch (reason) {
      setError(String(reason));
      setNewTitle(title);
    }
  }

  async function handleComplete(task: Task, done: boolean) {
    const optimistic = { ...task, done, status: done ? "Terminé" : "A faire" };
    updateLocal(optimistic);
    try {
      const saved = await backend.completeTask(task.id, done);
      updateLocal(saved);
      if (status?.configured) void handleSync();
    } catch (reason) {
      updateLocal(task);
      setError(String(reason));
    }
  }

  async function handleSave(task: Task) {
    updateLocal(task);
    try {
      const saved = await backend.saveTask(task);
      updateLocal(saved);
      const assignee = saved.assignee.trim();
      if (assignee) {
        setStatus((current) => {
          if (!current || current.settings.assigneeOptions.includes(assignee)) return current;
          return { ...current, settings: { ...current.settings, assigneeOptions: [...current.settings.assigneeOptions, assignee] } };
        });
      }
      if (status?.configured) void handleSync();
    } catch (reason) {
      setError(String(reason));
    }
  }

  function updateLocal(task: Task) {
    setTasks((current) => current.map((item) => item.id === task.id ? task : item));
    setSelected((current) => current?.id === task.id ? task : current);
  }

  async function handleDelete() {
    if (!deleteCandidate || deleting || syncing) return;
    const task = deleteCandidate;
    setDeleting(true);
    try {
      const pendingCount = await backend.deleteTask(task.id);
      setTasks((current) => current.filter((item) => item.id !== task.id));
      setSelected((current) => current?.id === task.id ? null : current);
      setStatus((current) => current ? { ...current, pendingCount } : current);
      setDeleteCandidate(null);
      if (status?.configured) void handleSync();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setDeleting(false);
    }
  }

  async function handleSync() {
    if (syncing || !status?.configured) return;
    setSyncing(true);
    try {
      await backend.sync();
      await load();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSyncing(false);
    }
  }

  const counts = useMemo(() => ({
    today: tasks.filter((task) => !task.done && (isToday(task.due) || isOverdue(task.due))).length,
    todo: tasks.filter((task) => !task.done).length,
    important: tasks.filter((task) => !task.done && ["P1 - Urgent & Important", "P2 - Important"].includes(task.priority)).length,
    later: tasks.filter((task) => !task.done && task.priority === "P4 - Plus tard").length,
    completed: tasks.filter((task) => task.done).length,
  }), [tasks]);

  const visibleTasks = useMemo(() => tasks
    .filter((task) => {
      if (activeView === "today") return !task.done && (isToday(task.due) || isOverdue(task.due));
      if (activeView === "todo") return !task.done;
      if (activeView === "important") return !task.done && ["P1 - Urgent & Important", "P2 - Important"].includes(task.priority);
      if (activeView === "later") return !task.done && task.priority === "P4 - Plus tard";
      return task.done;
    })
    .filter((task) => !query || `${task.title} ${task.description} ${task.comment}`.toLocaleLowerCase("fr").includes(query.toLocaleLowerCase("fr")))
    .sort((a, b) => Number(a.done) - Number(b.done) || (a.due ?? "9999").localeCompare(b.due ?? "9999") || b.updatedAt.localeCompare(a.updatedAt)),
  [tasks, activeView, query]);

  const currentTheme = themes.find((theme) => theme.id === themePreferences[activeView]) ?? themes[0];

  function chooseTheme(theme: AppTheme) {
    const next = { ...themePreferences, [activeView]: theme.id };
    setThemePreferences(next);
    saveThemePreferences(next);
  }

  if (!status) {
    return <div className="splash"><div className="spinner" />Chargement…</div>;
  }

  return (
    <div className="app-shell">
      <aside className={`sidebar ${sidebarOpen ? "open" : "collapsed"}`}>
        <div className="sidebar-top">
          <button className="icon-button" onClick={() => setSidebarOpen((value) => !value)} aria-label="Réduire le menu"><Menu size={20} /></button>
          {sidebarOpen && <span className="brand">Mes tâches</span>}
        </div>
        <nav>
          {navigation.map((item) => {
            const Icon = item.icon;
            return (
              <button key={item.key} className={`nav-item ${activeView === item.key ? "active" : ""}`} onClick={() => setActiveView(item.key)}>
                <Icon size={19} />
                {sidebarOpen && <><span>{item.label}</span><span className="nav-count">{counts[item.key]}</span></>}
              </button>
            );
          })}
        </nav>
        <div className="sidebar-bottom">
          <button className="nav-item" onClick={() => setSettingsOpen(true)}><SettingsIcon size={19} />{sidebarOpen && <span>Paramètres</span>}</button>
          {sidebarOpen && (
            <div className={`connection-state ${status.configured ? "connected" : "local"}`}>
              {status.configured ? <RefreshCw size={14} /> : <CloudOff size={14} />}
              <span>{status.configured ? `${status.pendingCount} en attente` : "Mode local"}</span>
            </div>
          )}
        </div>
      </aside>

      <main className={`main-content ${currentTheme.dark ? "theme-dark" : "theme-light"}`} style={themeStyle(currentTheme)}>
        <header className="content-header">
          <div>
            <h1>{navigation.find((item) => item.key === activeView)?.label}</h1>
            <p>{new Intl.DateTimeFormat("fr-FR", { weekday: "long", day: "numeric", month: "long" }).format(new Date())}</p>
          </div>
          <div className="header-actions">
            <button
              className="icon-button theme-button"
              onClick={() => setThemeOpen(true)}
              title="Choisir le thème"
              aria-label="Choisir le thème"
            >
              <Palette size={19} />
            </button>
            <label className="search-box"><Search size={18} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Rechercher" /></label>
            <button className="icon-button" disabled={!status.configured || syncing} onClick={() => void handleSync()} title="Synchroniser"><RefreshCw size={19} className={syncing ? "spin" : ""} /></button>
          </div>
        </header>

        {error && <div className="error-banner"><CircleAlert size={18} /><span>{error}</span><button onClick={() => setError(null)}><X size={17} /></button></div>}

        <form className="quick-add" onSubmit={handleAdd}>
          <span className="add-circle">+</span>
          <input autoFocus value={newTitle} onChange={(event) => setNewTitle(event.target.value)} placeholder="Ajouter une tâche" aria-label="Ajouter une tâche" />
          <span className="enter-hint">Entrée</span>
        </form>

        <section className="task-list">
          {visibleTasks.map((task) => <TaskRow key={task.id} task={task} onComplete={handleComplete} onSelect={setSelected} onSave={handleSave} />)}
          {visibleTasks.length === 0 && (
            <div className="empty-state"><Inbox size={42} /><h2>Tout est calme ici</h2><p>Ajoute une tâche avec le champ ci-dessus.</p></div>
          )}
        </section>
      </main>

      {selected && <DetailPanel task={selected} sourceOptions={status.settings.sourceOptions} assigneeOptions={status.settings.assigneeOptions} onClose={() => setSelected(null)} onSave={handleSave} onComplete={handleComplete} onDelete={() => setDeleteCandidate(selected)} deleteDisabled={syncing} />}
      {deleteCandidate && <DeleteDialog task={deleteCandidate} busy={deleting} syncInProgress={syncing} onCancel={() => !deleting && setDeleteCandidate(null)} onConfirm={() => void handleDelete()} />}
      {themeOpen && <ThemePanel activeTheme={currentTheme} onChoose={chooseTheme} onClose={() => setThemeOpen(false)} />}
      {settingsOpen && <SettingsPanel status={status} onClose={() => setSettingsOpen(false)} onSaved={async () => { setSettingsOpen(false); await load(); }} />}
    </div>
  );
}

function ThemePanel({ activeTheme, onChoose, onClose }: { activeTheme: AppTheme; onChoose: (theme: AppTheme) => void; onClose: () => void }) {
  const groups = ["Couleurs", "Dégradés", "Images"] as const;
  return (
    <div className="modal-backdrop" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="theme-modal">
        <div className="panel-header">
          <div><h2>Choisir un thème</h2><p>Le choix s’applique uniquement à la vue actuelle.</p></div>
          <button className="icon-button" onClick={onClose} aria-label="Fermer"><X size={20} /></button>
        </div>
        <div className="theme-body">
          {groups.map((group) => (
            <section className="theme-group" key={group}>
              <h3>{group}</h3>
              <div className={`theme-grid ${group === "Images" ? "image-grid" : ""}`}>
                {themes.filter((theme) => theme.group === group).map((theme) => (
                  <button
                    key={theme.id}
                    className={`theme-choice ${activeTheme.id === theme.id ? "selected" : ""}`}
                    onClick={() => onChoose(theme)}
                    title={theme.label}
                  >
                    <span className="theme-preview" style={{ background: theme.background, backgroundSize: theme.image ? "cover" : undefined, backgroundPosition: "center" }}>
                      {activeTheme.id === theme.id && <span className="theme-check"><Check size={17} /></span>}
                    </span>
                    <span>{theme.label}</span>
                  </button>
                ))}
              </div>
            </section>
          ))}
        </div>
        <div className="modal-actions"><span /><button className="primary-button" onClick={onClose}>Terminé</button></div>
      </section>
    </div>
  );
}

function TaskRow({ task, onComplete, onSelect, onSave }: { task: Task; onComplete: (task: Task, done: boolean) => void; onSelect: (task: Task) => void; onSave: (task: Task) => void }) {
  const important = ["P1 - Urgent & Important", "P2 - Important"].includes(task.priority);
  return (
    <article className={`task-row ${task.done ? "completed" : ""}`}>
      <button className={`check-button ${task.done ? "checked" : ""}`} onClick={() => onComplete(task, !task.done)} aria-label={task.done ? "Rouvrir" : "Terminer"}>{task.done && <Check size={15} />}</button>
      <button className="task-body" onClick={() => onSelect(task)}>
        <span className="task-title">{task.title}</span>
        <span className="task-meta">
          {formatDue(task.due) && <span className={isOverdue(task.due) && !task.done ? "overdue" : ""}>{formatDue(task.due)}</span>}
          {task.source && <span>{task.source}</span>}
          {task.dirty && <span className="pending-dot">À synchroniser</span>}
          {task.syncError && <span className="overdue">Erreur de synchronisation</span>}
        </span>
      </button>
      <button className={`star-button ${important ? "important" : ""}`} onClick={() => onSave({ ...task, priority: important ? "P3 - A faire" : "P2 - Important" })} aria-label="Basculer l’importance"><Star size={19} fill={important ? "currentColor" : "none"} /></button>
      <button className="row-chevron" onClick={() => onSelect(task)}><ChevronRight size={18} /></button>
    </article>
  );
}

function optionChoices(options: string[], current: string) {
  const names: string[] = [];
  for (const option of options) {
    const name = option.trim();
    if (name && !names.includes(name)) names.push(name);
  }
  if (current && !names.includes(current)) names.unshift(current);
  return names;
}

function OptionSelect({ value, options, onChange, allowCreate = false, emptyLabel, createPlaceholder }: { value: string; options: string[]; onChange: (value: string) => void; allowCreate?: boolean; emptyLabel: string; createPlaceholder?: string }) {
  const [creating, setCreating] = useState(false);
  const [draftName, setDraftName] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);

  function commit() {
    const name = draftName.trim();
    if (!name) {
      setCreating(false);
      setCreateError(null);
      return;
    }
    if (name.includes(",")) {
      setCreateError("Le nom ne peut pas contenir de virgule.");
      return;
    }
    onChange(name);
    setDraftName("");
    setCreateError(null);
    setCreating(false);
  }

  if (creating) {
    return (
      <>
        <span className="create-choice">
          <input autoFocus value={draftName} placeholder={createPlaceholder} aria-label={createPlaceholder} onChange={(event) => { setDraftName(event.target.value); setCreateError(null); }} onKeyDown={(event) => {
            if (event.key === "Enter") { event.preventDefault(); commit(); }
            if (event.key === "Escape") { setCreating(false); setDraftName(""); setCreateError(null); }
          }} />
          <button type="button" className="secondary-button" onClick={commit}>Ajouter</button>
        </span>
        {createError && <small className="field-hint">{createError}</small>}
      </>
    );
  }

  return (
    <>
      <select value={value} onChange={(event) => {
        if (event.target.value === "__create__") { setCreating(true); setDraftName(""); return; }
        onChange(event.target.value);
      }}>
        <option value="">{emptyLabel}</option>
        {optionChoices(options, value).map((option) => <option key={option} value={option}>{option}</option>)}
        {allowCreate && <option value="__create__">Ajouter…</option>}
      </select>
      {options.length === 0 && <small className="field-hint">Les choix viennent de Notion, après une connexion ou une synchronisation.</small>}
    </>
  );
}

function DetailPanel({ task, sourceOptions, assigneeOptions, onClose, onSave, onComplete, onDelete, deleteDisabled }: { task: Task; sourceOptions: string[]; assigneeOptions: string[]; onClose: () => void; onSave: (task: Task) => void; onComplete: (task: Task, done: boolean) => void; onDelete: () => void; deleteDisabled: boolean }) {
  const [draft, setDraft] = useState(task);
  const [assignees, setAssignees] = useState(assigneeOptions);
  useEffect(() => setDraft(task), [task]);
  useEffect(() => setAssignees(assigneeOptions), [assigneeOptions]);
  const update = <K extends keyof Task>(key: K, value: Task[K]) => setDraft((current) => ({ ...current, [key]: value }));
  const updateAssignee = (value: string) => {
    update("assignee", value);
    if (value && !assignees.includes(value)) setAssignees((current) => [...current, value]);
  };
  const saveAndClose = () => { onSave(draft); onClose(); };
  const toggleDone = () => {
    const done = !draft.done;
    const next = {
      ...draft,
      done,
      status: done ? "Terminé" : "A faire",
      completedAt: done ? new Date().toISOString().slice(0, 10) : null,
    };
    setDraft(next);
    onComplete(draft, done);
  };
  return (
    <aside className="detail-panel">
      <div className="panel-header"><span>Détails</span><button className="icon-button" onClick={onClose}><X size={20} /></button></div>
      <div className="panel-content">
        <div className="detail-title-row"><button className={`check-button ${draft.done ? "checked" : ""}`} onClick={toggleDone}>{draft.done && <Check size={15} />}</button><textarea value={draft.title} onChange={(event) => update("title", event.target.value)} rows={2} /></div>
        <Field label="Échéance"><input type="date" value={draft.due ?? ""} onChange={(event) => update("due", event.target.value || null)} /></Field>
        <Field label="Priorité"><select value={draft.priority} onChange={(event) => update("priority", event.target.value as Priority)}><option value="P1 - Urgent & Important">P1 — Urgent et important</option><option value="P2 - Important">P2 — Important</option><option value="P3 - A faire">P3 — À faire</option><option value="P4 - Plus tard">P4 — Plus tard</option></select></Field>
        <Field label="Statut"><select value={draft.status} onChange={(event) => update("status", event.target.value)}><option>A faire</option><option>En cours</option><option>Bloqué</option><option>Terminé</option><option>Abandonnée</option></select></Field>
        <Field label="Responsable"><OptionSelect value={draft.assignee} options={assignees} allowCreate emptyLabel="Aucun" createPlaceholder="Nom du responsable" onChange={updateAssignee} /></Field>
        <Field label="Source"><OptionSelect value={draft.source} options={sourceOptions} emptyLabel="Aucune" onChange={(value) => update("source", value)} /></Field>
        <Field label="Description"><textarea value={draft.description} onChange={(event) => update("description", event.target.value)} rows={5} placeholder="Ajouter des détails…" /></Field>
        <Field label="Commentaire"><textarea value={draft.comment} onChange={(event) => update("comment", event.target.value)} rows={4} /></Field>
      </div>
      <div className="panel-footer">
        <div className="panel-footer-start">
          <button className="danger-button delete-button" onClick={onDelete} disabled={deleteDisabled} title={deleteDisabled ? "Attends la fin de la synchronisation" : "Supprimer cette tâche"}><Trash2 size={16} />Supprimer</button>
          {draft.notionUrl && <a href={draft.notionUrl} target="_blank" rel="noreferrer">Ouvrir dans Notion</a>}
        </div>
        <button className="primary-button" onClick={saveAndClose}>Enregistrer</button>
      </div>
    </aside>
  );
}

function DeleteDialog({ task, busy, syncInProgress, onCancel, onConfirm }: { task: Task; busy: boolean; syncInProgress: boolean; onCancel: () => void; onConfirm: () => void }) {
  return (
    <div className="modal-backdrop" onMouseDown={(event) => event.target === event.currentTarget && onCancel()}>
      <section className="confirm-modal" role="alertdialog" aria-modal="true" aria-labelledby="delete-title" aria-describedby="delete-description">
        <div className="confirm-body">
          <span className="confirm-icon"><Trash2 size={24} /></span>
          <div>
            <h2 id="delete-title">Supprimer cette tâche ?</h2>
            <p id="delete-description"><strong>{task.title}</strong> sera supprimée de l’application{task.notionId ? " et placée dans la corbeille Notion lors de la synchronisation" : ""}.</p>
          </div>
        </div>
        <div className="modal-actions">
          {syncInProgress && <small>Synchronisation en cours…</small>}
          <span />
          <button className="secondary-button" onClick={onCancel} disabled={busy}>Annuler</button>
          <button className="danger-button confirm-danger" onClick={onConfirm} disabled={busy || syncInProgress}>{busy ? "Suppression…" : "Supprimer"}</button>
        </div>
      </section>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="field"><span>{label}</span>{children}</label>;
}

function SettingsPanel({ status, onClose, onSaved }: { status: AppStatus; onClose: () => void; onSaved: () => void }) {
  const [settings, setSettings] = useState(status.settings ?? initialSettings);
  const [token, setToken] = useState("");
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  async function connect() {
    if (!token.trim()) { setMessage("Saisis le jeton personnel Notion."); return; }
    setSaving(true);
    try { await backend.connect(token.trim(), settings); await onSaved(); }
    catch (reason) { setMessage(String(reason)); }
    finally { setSaving(false); }
  }
  async function saveLocal() {
    setSaving(true);
    try { await backend.saveSettings(settings); await onSaved(); }
    catch (reason) { setMessage(String(reason)); }
    finally { setSaving(false); }
  }
  async function disconnect() {
    setSaving(true);
    try { await backend.disconnect(); await onSaved(); }
    catch (reason) { setMessage(String(reason)); setSaving(false); }
  }
  return (
    <div className="modal-backdrop" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="settings-modal">
        <div className="panel-header"><div><h2>Connexion à Notion</h2><p>Le jeton reste chiffré sur cet appareil.</p></div><button className="icon-button" onClick={onClose}><X size={20} /></button></div>
        <div className="settings-body">
          <Field label="Identifiant de la source de données"><input value={settings.dataSourceId} onChange={(event) => setSettings({ ...settings, dataSourceId: event.target.value })} /></Field>
          <Field label={status.tokenStored ? "Nouveau jeton Notion (laisser vide pour conserver l’actuel)" : "Jeton personnel Notion"}><input type="password" value={token} onChange={(event) => setToken(event.target.value)} placeholder="ntn_…" autoComplete="off" /></Field>
          <div className="settings-grid">
            <Field label="Statut par défaut"><input value={settings.defaultStatus} onChange={(event) => setSettings({ ...settings, defaultStatus: event.target.value })} /></Field>
            <Field label="Responsable par défaut"><OptionSelect value={settings.defaultAssignee} options={settings.assigneeOptions} allowCreate emptyLabel="Aucun" createPlaceholder="Nom du responsable" onChange={(value) => setSettings({ ...settings, defaultAssignee: value, assigneeOptions: value && !settings.assigneeOptions.includes(value) ? [...settings.assigneeOptions, value] : settings.assigneeOptions })} /></Field>
            <Field label="Source par défaut"><OptionSelect value={settings.defaultSource} options={settings.sourceOptions} emptyLabel="Aucune" onChange={(value) => setSettings({ ...settings, defaultSource: value })} /></Field>
            <Field label="Synchronisation (minutes)"><input type="number" min={1} max={60} value={settings.syncIntervalMinutes} onChange={(event) => setSettings({ ...settings, syncIntervalMinutes: Number(event.target.value) })} /></Field>
          </div>
          {message && <div className="inline-message">{message}</div>}
          <p className="settings-note">Pour utiliser la synchronisation, crée un jeton personnel dans Notion et donne-lui accès à la base Tâches. L’application reste utilisable localement sans jeton.</p>
        </div>
        <div className="modal-actions">{status.tokenStored && <button className="danger-button" disabled={saving} onClick={() => void disconnect()}>Déconnecter</button>}<span /><button className="secondary-button" disabled={saving} onClick={() => void saveLocal()}>Enregistrer localement</button><button className="primary-button" disabled={saving || !token.trim()} onClick={() => void connect()}>{saving ? "Vérification…" : "Vérifier et connecter"}</button></div>
      </section>
    </div>
  );
}
