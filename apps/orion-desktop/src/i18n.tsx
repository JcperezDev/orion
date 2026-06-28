import { createContext, useContext, useState, useCallback, type ReactNode } from 'react'

export type Lang = 'en' | 'es' | 'pt' | 'fr' | 'de' | 'it' | 'zh' | 'ja' | 'ko' | 'ru'

export const LANGS: Array<{ id: Lang; label: string }> = [
  { id: 'en', label: 'English' },
  { id: 'es', label: 'Español' },
  { id: 'pt', label: 'Português' },
  { id: 'fr', label: 'Français' },
  { id: 'de', label: 'Deutsch' },
  { id: 'it', label: 'Italiano' },
  { id: 'zh', label: '中文' },
  { id: 'ja', label: '日本語' },
  { id: 'ko', label: '한국어' },
  { id: 'ru', label: 'Русский' },
]

type Dict = Record<string, string>

// English is the source of truth; missing keys fall back to it.
const en: Dict = {
  'welcome.subtitle': 'AI coding assistant',
  'welcome.shortcuts': 'Shortcuts',
  'welcome.send': 'Send message',
  'welcome.newline': 'New line',
  'welcome.openSettings': 'Open settings',
  'welcome.modes': 'Modes',
  'mode.build.desc': 'Direct model response, no tools',
  'mode.plan.desc': 'Read-only: explore the code without editing',
  'mode.agent.desc': 'ORION uses tools: read, write, bash, MCP',
  'sidebar.newSession': 'New session',
  'sidebar.sessions': 'Sessions',
  'sidebar.search': 'Search sessions…',
  'sidebar.settings': 'Settings',
  'sidebar.noSessions': 'No sessions yet',
  'sidebar.noResults': 'No results',
  'sidebar.loading': 'Loading…',
  'input.placeholder': 'Type a message or / for commands…',
  'input.connectFirst': 'Connect a provider first…',
  'input.context': '+ Context',
  'header.selectModel': 'Select model',
  'header.loadingSession': 'Loading session…',
  'settings.backToChat': 'Back to chat',
}

const es: Dict = {
  'welcome.subtitle': 'Asistente de programación con IA',
  'welcome.shortcuts': 'Atajos',
  'welcome.send': 'Enviar mensaje',
  'welcome.newline': 'Nueva línea',
  'welcome.openSettings': 'Abrir ajustes',
  'welcome.modes': 'Modos',
  'mode.build.desc': 'Respuesta directa del modelo, sin herramientas',
  'mode.plan.desc': 'Solo lectura: explora el código sin editarlo',
  'mode.agent.desc': 'ORION usa herramientas: leer, escribir, bash, MCP',
  'sidebar.newSession': 'Nueva sesión',
  'sidebar.sessions': 'Sesiones',
  'sidebar.search': 'Buscar sesiones…',
  'sidebar.settings': 'Ajustes',
  'sidebar.noSessions': 'No hay sesiones',
  'sidebar.noResults': 'Sin resultados',
  'sidebar.loading': 'Cargando…',
  'input.placeholder': 'Escribe un mensaje o / para comandos…',
  'input.connectFirst': 'Conecta un proveedor primero…',
  'input.context': '+ Contexto',
  'header.selectModel': 'Elegir modelo',
  'header.loadingSession': 'Cargando sesión…',
  'settings.backToChat': 'Volver al chat',
}

const pt: Dict = {
  'welcome.subtitle': 'Assistente de programação com IA',
  'welcome.shortcuts': 'Atalhos',
  'welcome.send': 'Enviar mensagem',
  'welcome.newline': 'Nova linha',
  'welcome.openSettings': 'Abrir configurações',
  'welcome.modes': 'Modos',
  'mode.build.desc': 'Resposta direta do modelo, sem ferramentas',
  'mode.plan.desc': 'Somente leitura: explore o código sem editar',
  'mode.agent.desc': 'ORION usa ferramentas: ler, escrever, bash, MCP',
  'sidebar.newSession': 'Nova sessão',
  'sidebar.sessions': 'Sessões',
  'sidebar.search': 'Buscar sessões…',
  'sidebar.settings': 'Configurações',
  'sidebar.noSessions': 'Nenhuma sessão ainda',
  'sidebar.noResults': 'Sem resultados',
  'sidebar.loading': 'Carregando…',
  'input.placeholder': 'Digite uma mensagem ou / para comandos…',
  'input.connectFirst': 'Conecte um provedor primeiro…',
  'input.context': '+ Contexto',
  'header.selectModel': 'Escolher modelo',
  'header.loadingSession': 'Carregando sessão…',
  'settings.backToChat': 'Voltar ao chat',
}

const fr: Dict = {
  'welcome.subtitle': 'Assistant de codage par IA',
  'welcome.shortcuts': 'Raccourcis',
  'welcome.send': 'Envoyer le message',
  'welcome.newline': 'Nouvelle ligne',
  'welcome.openSettings': 'Ouvrir les paramètres',
  'welcome.modes': 'Modes',
  'mode.build.desc': 'Réponse directe du modèle, sans outils',
  'mode.plan.desc': 'Lecture seule : explorer le code sans le modifier',
  'mode.agent.desc': 'ORION utilise des outils : lire, écrire, bash, MCP',
  'sidebar.newSession': 'Nouvelle session',
  'sidebar.sessions': 'Sessions',
  'sidebar.search': 'Rechercher des sessions…',
  'sidebar.settings': 'Paramètres',
  'sidebar.noSessions': 'Aucune session',
  'sidebar.noResults': 'Aucun résultat',
  'sidebar.loading': 'Chargement…',
  'input.placeholder': 'Tapez un message ou / pour les commandes…',
  'input.connectFirst': 'Connectez d’abord un fournisseur…',
  'input.context': '+ Contexte',
  'header.selectModel': 'Choisir le modèle',
  'header.loadingSession': 'Chargement de la session…',
  'settings.backToChat': 'Retour au chat',
}

const de: Dict = {
  'welcome.subtitle': 'KI-Programmierassistent',
  'welcome.shortcuts': 'Tastenkürzel',
  'welcome.send': 'Nachricht senden',
  'welcome.newline': 'Neue Zeile',
  'welcome.openSettings': 'Einstellungen öffnen',
  'welcome.modes': 'Modi',
  'mode.build.desc': 'Direkte Modellantwort, keine Werkzeuge',
  'mode.plan.desc': 'Nur lesen: Code erkunden ohne zu bearbeiten',
  'mode.agent.desc': 'ORION nutzt Werkzeuge: lesen, schreiben, bash, MCP',
  'sidebar.newSession': 'Neue Sitzung',
  'sidebar.sessions': 'Sitzungen',
  'sidebar.search': 'Sitzungen suchen…',
  'sidebar.settings': 'Einstellungen',
  'sidebar.noSessions': 'Noch keine Sitzungen',
  'sidebar.noResults': 'Keine Ergebnisse',
  'sidebar.loading': 'Lädt…',
  'input.placeholder': 'Nachricht eingeben oder / für Befehle…',
  'input.connectFirst': 'Zuerst einen Anbieter verbinden…',
  'input.context': '+ Kontext',
  'header.selectModel': 'Modell wählen',
  'header.loadingSession': 'Sitzung wird geladen…',
  'settings.backToChat': 'Zurück zum Chat',
}

const it: Dict = {
  'welcome.subtitle': 'Assistente di programmazione IA',
  'welcome.shortcuts': 'Scorciatoie',
  'welcome.send': 'Invia messaggio',
  'welcome.newline': 'Nuova riga',
  'welcome.openSettings': 'Apri impostazioni',
  'welcome.modes': 'Modalità',
  'mode.build.desc': 'Risposta diretta del modello, senza strumenti',
  'mode.plan.desc': 'Sola lettura: esplora il codice senza modificarlo',
  'mode.agent.desc': 'ORION usa strumenti: leggere, scrivere, bash, MCP',
  'sidebar.newSession': 'Nuova sessione',
  'sidebar.sessions': 'Sessioni',
  'sidebar.search': 'Cerca sessioni…',
  'sidebar.settings': 'Impostazioni',
  'sidebar.noSessions': 'Nessuna sessione',
  'sidebar.noResults': 'Nessun risultato',
  'sidebar.loading': 'Caricamento…',
  'input.placeholder': 'Scrivi un messaggio o / per i comandi…',
  'input.connectFirst': 'Collega prima un provider…',
  'input.context': '+ Contesto',
  'header.selectModel': 'Scegli modello',
  'header.loadingSession': 'Caricamento sessione…',
  'settings.backToChat': 'Torna alla chat',
}

const zh: Dict = {
  'welcome.subtitle': 'AI 编程助手',
  'welcome.shortcuts': '快捷键',
  'welcome.send': '发送消息',
  'welcome.newline': '换行',
  'welcome.openSettings': '打开设置',
  'welcome.modes': '模式',
  'mode.build.desc': '模型直接回复，不使用工具',
  'mode.plan.desc': '只读：浏览代码而不修改',
  'mode.agent.desc': 'ORION 使用工具：读取、写入、bash、MCP',
  'sidebar.newSession': '新会话',
  'sidebar.sessions': '会话',
  'sidebar.search': '搜索会话…',
  'sidebar.settings': '设置',
  'sidebar.noSessions': '暂无会话',
  'sidebar.noResults': '无结果',
  'sidebar.loading': '加载中…',
  'input.placeholder': '输入消息，或输入 / 调用命令…',
  'input.connectFirst': '请先连接一个服务商…',
  'input.context': '+ 上下文',
  'header.selectModel': '选择模型',
  'header.loadingSession': '正在加载会话…',
  'settings.backToChat': '返回聊天',
}

const ja: Dict = {
  'welcome.subtitle': 'AI コーディングアシスタント',
  'welcome.shortcuts': 'ショートカット',
  'welcome.send': 'メッセージを送信',
  'welcome.newline': '改行',
  'welcome.openSettings': '設定を開く',
  'welcome.modes': 'モード',
  'mode.build.desc': 'モデルの直接応答、ツールなし',
  'mode.plan.desc': '読み取り専用：編集せずコードを確認',
  'mode.agent.desc': 'ORION はツールを使用：読み取り、書き込み、bash、MCP',
  'sidebar.newSession': '新しいセッション',
  'sidebar.sessions': 'セッション',
  'sidebar.search': 'セッションを検索…',
  'sidebar.settings': '設定',
  'sidebar.noSessions': 'セッションはまだありません',
  'sidebar.noResults': '結果なし',
  'sidebar.loading': '読み込み中…',
  'input.placeholder': 'メッセージを入力、または / でコマンド…',
  'input.connectFirst': '先にプロバイダーを接続してください…',
  'input.context': '+ コンテキスト',
  'header.selectModel': 'モデルを選択',
  'header.loadingSession': 'セッションを読み込み中…',
  'settings.backToChat': 'チャットに戻る',
}

const ko: Dict = {
  'welcome.subtitle': 'AI 코딩 어시스턴트',
  'welcome.shortcuts': '단축키',
  'welcome.send': '메시지 보내기',
  'welcome.newline': '줄 바꿈',
  'welcome.openSettings': '설정 열기',
  'welcome.modes': '모드',
  'mode.build.desc': '모델 직접 응답, 도구 없음',
  'mode.plan.desc': '읽기 전용: 코드를 수정하지 않고 탐색',
  'mode.agent.desc': 'ORION이 도구 사용: 읽기, 쓰기, bash, MCP',
  'sidebar.newSession': '새 세션',
  'sidebar.sessions': '세션',
  'sidebar.search': '세션 검색…',
  'sidebar.settings': '설정',
  'sidebar.noSessions': '아직 세션이 없습니다',
  'sidebar.noResults': '결과 없음',
  'sidebar.loading': '불러오는 중…',
  'input.placeholder': '메시지를 입력하거나 / 로 명령…',
  'input.connectFirst': '먼저 공급자를 연결하세요…',
  'input.context': '+ 컨텍스트',
  'header.selectModel': '모델 선택',
  'header.loadingSession': '세션 불러오는 중…',
  'settings.backToChat': '채팅으로 돌아가기',
}

const ru: Dict = {
  'welcome.subtitle': 'ИИ-ассистент по программированию',
  'welcome.shortcuts': 'Горячие клавиши',
  'welcome.send': 'Отправить сообщение',
  'welcome.newline': 'Новая строка',
  'welcome.openSettings': 'Открыть настройки',
  'welcome.modes': 'Режимы',
  'mode.build.desc': 'Прямой ответ модели, без инструментов',
  'mode.plan.desc': 'Только чтение: изучение кода без изменений',
  'mode.agent.desc': 'ORION использует инструменты: чтение, запись, bash, MCP',
  'sidebar.newSession': 'Новая сессия',
  'sidebar.sessions': 'Сессии',
  'sidebar.search': 'Поиск сессий…',
  'sidebar.settings': 'Настройки',
  'sidebar.noSessions': 'Пока нет сессий',
  'sidebar.noResults': 'Нет результатов',
  'sidebar.loading': 'Загрузка…',
  'input.placeholder': 'Введите сообщение или / для команд…',
  'input.connectFirst': 'Сначала подключите провайдера…',
  'input.context': '+ Контекст',
  'header.selectModel': 'Выбрать модель',
  'header.loadingSession': 'Загрузка сессии…',
  'settings.backToChat': 'Назад в чат',
}

const TRANSLATIONS: Record<Lang, Dict> = { en, es, pt, fr, de, it, zh, ja, ko, ru }

interface LangCtx {
  lang: Lang
  setLang: (l: Lang) => void
  t: (key: string) => string
}

const Ctx = createContext<LangCtx>({ lang: 'en', setLang: () => {}, t: (k) => k })

function readStoredLang(): Lang {
  const saved = (typeof localStorage !== 'undefined' && localStorage.getItem('orion-lang')) as Lang | null
  return saved && TRANSLATIONS[saved] ? saved : 'en'
}

export function LangProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(readStoredLang)
  const setLang = useCallback((l: Lang) => {
    setLangState(l)
    try { localStorage.setItem('orion-lang', l) } catch { /* ignore */ }
  }, [])
  const t = useCallback(
    (key: string) => TRANSLATIONS[lang]?.[key] ?? en[key] ?? key,
    [lang]
  )
  return <Ctx.Provider value={{ lang, setLang, t }}>{children}</Ctx.Provider>
}

export function useLang() {
  return useContext(Ctx)
}

export function useT() {
  return useContext(Ctx).t
}
