import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import { LangProvider } from './i18n'
import { applyStoredTheme } from './components/SettingsView'
import './index.css'
import './styles/titlebar.css'
import './styles/permission.css'
import './styles/sidebar.css'
import './styles/chat.css'
import './styles/onboarding.css'

// Apply the saved theme before the first paint to avoid a flash of the default.
applyStoredTheme()

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <LangProvider>
      <App />
    </LangProvider>
  </React.StrictMode>,
)
