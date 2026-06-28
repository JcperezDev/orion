import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
import './styles/titlebar.css'
import './styles/permission.css'
import './styles/sidebar.css'
import './styles/chat.css'
import './styles/onboarding.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
