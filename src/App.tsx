import { NavLink, Outlet } from "react-router-dom";

export function AppShell() {
  return (
    <div className="flex h-full">
      <aside className="w-56 shrink-0 border-r border-slate-200 bg-white p-4 dark:bg-slate-900 dark:border-slate-800">
        <div className="mb-6">
          <div className="text-lg font-semibold">rpstr</div>
          <div className="text-xs text-slate-500">asystent psychiatry</div>
        </div>
        <nav className="flex flex-col gap-1 text-sm">
          <NavItem to="/app/visits">Wizyty</NavItem>
          <NavItem to="/app/visits/new">Nowa wizyta</NavItem>
          <NavItem to="/app/settings">Ustawienia</NavItem>
        </nav>
      </aside>
      <main className="flex-1 overflow-auto p-6">
        <Outlet />
      </main>
    </div>
  );
}

function NavItem({ to, children }: { to: string; children: React.ReactNode }) {
  return (
    <NavLink
      to={to}
      end
      className={({ isActive }) =>
        `rounded-md px-3 py-2 transition ${
          isActive
            ? "bg-brand-600 text-white"
            : "text-slate-700 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
        }`
      }
    >
      {children}
    </NavLink>
  );
}
