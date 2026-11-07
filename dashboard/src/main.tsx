import { FormEvent, StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "./style.css";

function App() {
  const [token, setToken] = useState(() => localStorage.getItem("fasthop-admin-token") ?? "");
  const [users, setUsers] = useState<Array<{ telegram_id: number; status: string; expires_at: string; token: string }>>([]);
  const [logs, setLogs] = useState<Array<[number, string, string, string | null, string]>>([]);
  const [error, setError] = useState("");
  const api = async (path: string, options: RequestInit = {}) => {
    const response = await fetch(path, { ...options, headers: { authorization: `Bearer ${token}`, "content-type": "application/json", ...(options.headers ?? {}) } });
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
    return response.status === 204 ? null : response.json();
  };
  const load = async () => {
    try { setError(""); setUsers(await api("/api/admin/subscriptions")); setLogs(await api("/api/admin/logs")); }
    catch (e) { setError(e instanceof Error ? e.message : "Unable to load admin data"); }
  };
  useEffect(() => { if (token) void load(); }, [token]);
  const saveToken = (event: FormEvent) => { event.preventDefault(); localStorage.setItem("fasthop-admin-token", token); void load(); };
  return <main><p className="eyebrow">FASTHOP</p><h1>DNS access dashboard</h1><p className="muted">Manage subscriptions and query activity.</p>
    <form onSubmit={saveToken}><input value={token} onChange={e => setToken(e.target.value)} placeholder="ADMIN_TOKEN" type="password" /><button>Connect</button></form>
    {error && <p className="error">{error}</p>}
    <section><strong>Service status</strong><span className="status">● Operational</span></section>
    <section className="column"><h2>Subscriptions ({users.length})</h2>{users.map(user => <div className="row" key={user.telegram_id}><span>{user.telegram_id}</span><span>{user.status}</span><small>{new Date(user.expires_at).toLocaleString()}</small></div>)}</section>
    <section className="column"><h2>Recent queries ({logs.length})</h2>{logs.map(log => <div className="row" key={log[0]}><span>{log[1]}</span><small>{log[3] ?? "unknown"} · {new Date(log[4]).toLocaleString()}</small></div>)}</section>
  </main>;
}

createRoot(document.getElementById("root")!).render(<StrictMode><App /></StrictMode>);
