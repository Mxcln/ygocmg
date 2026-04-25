import { useEffect, useState } from "react";
import { appApi } from "../shared/api/app";
import type { GlobalConfig, WorkspaceRegistryFile } from "../shared/contracts/app";

interface AppBootstrapState {
  config: GlobalConfig | null;
  recentWorkspaces: WorkspaceRegistryFile | null;
}

export function App() {
  const [state, setState] = useState<AppBootstrapState>({
    config: null,
    recentWorkspaces: null,
  });
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;

    async function bootstrap() {
      try {
        const config = await appApi.initialize();
        const recentWorkspaces = await appApi.listRecentWorkspaces();
        if (!active) {
          return;
        }
        setState({ config, recentWorkspaces });
      } catch (nextError) {
        if (!active) {
          return;
        }
        setError(formatError(nextError));
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    }

    void bootstrap();

    return () => {
      active = false;
    };
  }, []);

  return (
    <main className="app-shell">
      <section className="hero-card">
        <p className="eyebrow">YGOCMG / P0</p>
        <h1>作者态内核已经接入 Tauri 壳层</h1>
        <p className="hero-copy">
          这个最小页面先验证前后端链路：初始化应用、读取程序级配置、获取 recent
          workspaces。
        </p>

        {loading ? <p className="status">正在初始化应用…</p> : null}
        {error ? <p className="status error">{error}</p> : null}

        {!loading && !error && state.config && state.recentWorkspaces ? (
          <div className="grid">
            <article className="panel">
              <h2>Global Config</h2>
              <dl className="kv-list">
                <div>
                  <dt>UI Language</dt>
                  <dd>{state.config.app_language}</dd>
                </div>
                <div>
                  <dt>YGOPro Path</dt>
                  <dd>{state.config.ygopro_path ?? "未设置"}</dd>
                </div>
                <div>
                  <dt>Workspace Root</dt>
                  <dd>{state.config.default_workspace_root ?? "未设置"}</dd>
                </div>
                <div>
                  <dt>Code Range</dt>
                  <dd>
                    {state.config.custom_code_recommended_min} -{" "}
                    {state.config.custom_code_recommended_max}
                  </dd>
                </div>
              </dl>
            </article>

            <article className="panel">
              <h2>Recent Workspaces</h2>
              {state.recentWorkspaces.workspaces.length === 0 ? (
                <p className="empty">当前还没有 recent workspace 记录。</p>
              ) : (
                <ul className="workspace-list">
                  {state.recentWorkspaces.workspaces.map((workspace) => (
                    <li key={workspace.workspace_id}>
                      <span>{workspace.name_cache ?? workspace.workspace_id}</span>
                      <code>{workspace.path}</code>
                    </li>
                  ))}
                </ul>
              )}
            </article>
          </div>
        ) : null}
      </section>
    </main>
  );
}

function formatError(error: unknown) {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error) {
    const appError = error as { code: string; message: string };
    return `${appError.code}: ${appError.message}`;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "发生了未知错误。";
}
