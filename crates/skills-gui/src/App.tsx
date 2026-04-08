import { BrowserRouter, Routes, Route } from "react-router"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { Toaster } from "sonner"
import { ThemeProvider } from "./lib/theme"
import { AppLayout } from "./layouts/AppLayout"
import { Dashboard } from "./pages/Dashboard"
import { Skills } from "./pages/Skills"
import { Profiles } from "./pages/Profiles"
import { Projects } from "./pages/Projects"
import { Agents } from "./pages/Agents"
import { ActivityLog } from "./pages/ActivityLog"
import { Settings } from "./pages/Settings"

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 2_000,
    },
  },
})

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <BrowserRouter>
          <Routes>
            <Route element={<AppLayout />}>
              <Route path="/" element={<Dashboard />} />
              <Route path="/skills" element={<Skills />} />
              <Route path="/profiles" element={<Profiles />} />
              <Route path="/projects" element={<Projects />} />
              <Route path="/agents" element={<Agents />} />
              <Route path="/activity" element={<ActivityLog />} />
              <Route path="/settings" element={<Settings />} />
            </Route>
          </Routes>
        </BrowserRouter>
        <Toaster richColors theme="system" />
      </ThemeProvider>
    </QueryClientProvider>
  )
}

export default App
