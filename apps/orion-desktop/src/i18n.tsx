import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react'

export type Lang =
  | 'en' | 'es' | 'pt' | 'fr' | 'de' | 'it' | 'zh' | 'ja' | 'ko' | 'ru'
  | 'nl' | 'pl' | 'tr' | 'uk' | 'ar' | 'he' | 'fa' | 'hi' | 'id' | 'vi'
  | 'th' | 'cs' | 'sv' | 'el' | 'ro' | 'hu' | 'ca' | 'ms' | 'da' | 'fi' | 'zhTW'

export const LANGS: Array<{ id: Lang; label: string }> = [
  { id: 'en', label: 'English' },
  { id: 'es', label: 'Español' },
  { id: 'pt', label: 'Português' },
  { id: 'fr', label: 'Français' },
  { id: 'de', label: 'Deutsch' },
  { id: 'it', label: 'Italiano' },
  { id: 'nl', label: 'Nederlands' },
  { id: 'pl', label: 'Polski' },
  { id: 'ro', label: 'Română' },
  { id: 'cs', label: 'Čeština' },
  { id: 'hu', label: 'Magyar' },
  { id: 'el', label: 'Ελληνικά' },
  { id: 'sv', label: 'Svenska' },
  { id: 'da', label: 'Dansk' },
  { id: 'fi', label: 'Suomi' },
  { id: 'ca', label: 'Català' },
  { id: 'tr', label: 'Türkçe' },
  { id: 'uk', label: 'Українська' },
  { id: 'ru', label: 'Русский' },
  { id: 'ar', label: 'العربية' },
  { id: 'he', label: 'עברית' },
  { id: 'fa', label: 'فارسی' },
  { id: 'hi', label: 'हिन्दी' },
  { id: 'id', label: 'Bahasa Indonesia' },
  { id: 'ms', label: 'Bahasa Melayu' },
  { id: 'vi', label: 'Tiếng Việt' },
  { id: 'th', label: 'ไทย' },
  { id: 'zh', label: '中文 (简体)' },
  { id: 'zhTW', label: '中文 (繁體)' },
  { id: 'ja', label: '日本語' },
  { id: 'ko', label: '한국어' },
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

const nl: Dict = {
  'welcome.subtitle': 'AI-programmeerassistent', 'welcome.shortcuts': 'Sneltoetsen', 'welcome.send': 'Bericht verzenden', 'welcome.newline': 'Nieuwe regel', 'welcome.openSettings': 'Instellingen openen', 'welcome.modes': 'Modi',
  'mode.build.desc': 'Direct modelantwoord, geen tools', 'mode.plan.desc': 'Alleen-lezen: code verkennen zonder te bewerken', 'mode.agent.desc': 'ORION gebruikt tools: lezen, schrijven, bash, MCP',
  'sidebar.newSession': 'Nieuwe sessie', 'sidebar.sessions': 'Sessies', 'sidebar.search': 'Sessies zoeken…', 'sidebar.settings': 'Instellingen', 'sidebar.noSessions': 'Nog geen sessies', 'sidebar.noResults': 'Geen resultaten', 'sidebar.loading': 'Laden…',
  'input.placeholder': 'Typ een bericht of / voor opdrachten…', 'input.connectFirst': 'Verbind eerst een provider…', 'input.context': '+ Context', 'header.selectModel': 'Model kiezen', 'header.loadingSession': 'Sessie laden…', 'settings.backToChat': 'Terug naar chat',
}
const pl: Dict = {
  'welcome.subtitle': 'Asystent programowania AI', 'welcome.shortcuts': 'Skróty', 'welcome.send': 'Wyślij wiadomość', 'welcome.newline': 'Nowy wiersz', 'welcome.openSettings': 'Otwórz ustawienia', 'welcome.modes': 'Tryby',
  'mode.build.desc': 'Bezpośrednia odpowiedź modelu, bez narzędzi', 'mode.plan.desc': 'Tylko do odczytu: przeglądaj kod bez edycji', 'mode.agent.desc': 'ORION używa narzędzi: odczyt, zapis, bash, MCP',
  'sidebar.newSession': 'Nowa sesja', 'sidebar.sessions': 'Sesje', 'sidebar.search': 'Szukaj sesji…', 'sidebar.settings': 'Ustawienia', 'sidebar.noSessions': 'Brak sesji', 'sidebar.noResults': 'Brak wyników', 'sidebar.loading': 'Ładowanie…',
  'input.placeholder': 'Wpisz wiadomość lub / aby uzyskać polecenia…', 'input.connectFirst': 'Najpierw połącz dostawcę…', 'input.context': '+ Kontekst', 'header.selectModel': 'Wybierz model', 'header.loadingSession': 'Ładowanie sesji…', 'settings.backToChat': 'Powrót do czatu',
}
const tr: Dict = {
  'welcome.subtitle': 'Yapay zekâ kodlama asistanı', 'welcome.shortcuts': 'Kısayollar', 'welcome.send': 'Mesaj gönder', 'welcome.newline': 'Yeni satır', 'welcome.openSettings': 'Ayarları aç', 'welcome.modes': 'Modlar',
  'mode.build.desc': 'Doğrudan model yanıtı, araç yok', 'mode.plan.desc': 'Salt okunur: kodu düzenlemeden incele', 'mode.agent.desc': 'ORION araçları kullanır: oku, yaz, bash, MCP',
  'sidebar.newSession': 'Yeni oturum', 'sidebar.sessions': 'Oturumlar', 'sidebar.search': 'Oturum ara…', 'sidebar.settings': 'Ayarlar', 'sidebar.noSessions': 'Henüz oturum yok', 'sidebar.noResults': 'Sonuç yok', 'sidebar.loading': 'Yükleniyor…',
  'input.placeholder': 'Bir mesaj yazın veya komutlar için /…', 'input.connectFirst': 'Önce bir sağlayıcı bağlayın…', 'input.context': '+ Bağlam', 'header.selectModel': 'Model seç', 'header.loadingSession': 'Oturum yükleniyor…', 'settings.backToChat': 'Sohbete dön',
}
const uk: Dict = {
  'welcome.subtitle': 'ШІ-асистент із програмування', 'welcome.shortcuts': 'Комбінації клавіш', 'welcome.send': 'Надіслати повідомлення', 'welcome.newline': 'Новий рядок', 'welcome.openSettings': 'Відкрити налаштування', 'welcome.modes': 'Режими',
  'mode.build.desc': 'Пряма відповідь моделі, без інструментів', 'mode.plan.desc': 'Лише читання: перегляд коду без редагування', 'mode.agent.desc': 'ORION використовує інструменти: читання, запис, bash, MCP',
  'sidebar.newSession': 'Нова сесія', 'sidebar.sessions': 'Сесії', 'sidebar.search': 'Пошук сесій…', 'sidebar.settings': 'Налаштування', 'sidebar.noSessions': 'Ще немає сесій', 'sidebar.noResults': 'Немає результатів', 'sidebar.loading': 'Завантаження…',
  'input.placeholder': 'Введіть повідомлення або / для команд…', 'input.connectFirst': 'Спершу підключіть провайдера…', 'input.context': '+ Контекст', 'header.selectModel': 'Вибрати модель', 'header.loadingSession': 'Завантаження сесії…', 'settings.backToChat': 'Назад до чату',
}
const ar: Dict = {
  'welcome.subtitle': 'مساعد البرمجة بالذكاء الاصطناعي', 'welcome.shortcuts': 'الاختصارات', 'welcome.send': 'إرسال الرسالة', 'welcome.newline': 'سطر جديد', 'welcome.openSettings': 'فتح الإعدادات', 'welcome.modes': 'الأوضاع',
  'mode.build.desc': 'رد مباشر من النموذج، بدون أدوات', 'mode.plan.desc': 'للقراءة فقط: استكشف الكود دون تعديله', 'mode.agent.desc': 'يستخدم ORION الأدوات: قراءة، كتابة، bash، MCP',
  'sidebar.newSession': 'جلسة جديدة', 'sidebar.sessions': 'الجلسات', 'sidebar.search': 'البحث في الجلسات…', 'sidebar.settings': 'الإعدادات', 'sidebar.noSessions': 'لا توجد جلسات بعد', 'sidebar.noResults': 'لا نتائج', 'sidebar.loading': 'جارٍ التحميل…',
  'input.placeholder': 'اكتب رسالة أو / للأوامر…', 'input.connectFirst': 'اربط مزودًا أولاً…', 'input.context': '+ سياق', 'header.selectModel': 'اختر النموذج', 'header.loadingSession': 'جارٍ تحميل الجلسة…', 'settings.backToChat': 'العودة إلى الدردشة',
}
const he: Dict = {
  'welcome.subtitle': 'עוזר תכנות מבוסס בינה מלאכותית', 'welcome.shortcuts': 'קיצורי מקלדת', 'welcome.send': 'שליחת הודעה', 'welcome.newline': 'שורה חדשה', 'welcome.openSettings': 'פתיחת הגדרות', 'welcome.modes': 'מצבים',
  'mode.build.desc': 'תשובה ישירה מהמודל, ללא כלים', 'mode.plan.desc': 'קריאה בלבד: חקירת הקוד ללא עריכה', 'mode.agent.desc': 'ORION משתמש בכלים: קריאה, כתיבה, bash, MCP',
  'sidebar.newSession': 'שיחה חדשה', 'sidebar.sessions': 'שיחות', 'sidebar.search': 'חיפוש שיחות…', 'sidebar.settings': 'הגדרות', 'sidebar.noSessions': 'אין שיחות עדיין', 'sidebar.noResults': 'אין תוצאות', 'sidebar.loading': 'טוען…',
  'input.placeholder': 'הקלד הודעה או / לפקודות…', 'input.connectFirst': 'חבר ספק תחילה…', 'input.context': '+ הקשר', 'header.selectModel': 'בחר מודל', 'header.loadingSession': 'טוען שיחה…', 'settings.backToChat': 'חזרה לצ׳אט',
}
const fa: Dict = {
  'welcome.subtitle': 'دستیار برنامه‌نویسی هوش مصنوعی', 'welcome.shortcuts': 'میان‌برها', 'welcome.send': 'ارسال پیام', 'welcome.newline': 'خط جدید', 'welcome.openSettings': 'باز کردن تنظیمات', 'welcome.modes': 'حالت‌ها',
  'mode.build.desc': 'پاسخ مستقیم مدل، بدون ابزار', 'mode.plan.desc': 'فقط‌خواندنی: بررسی کد بدون ویرایش', 'mode.agent.desc': 'ORION از ابزارها استفاده می‌کند: خواندن، نوشتن، bash، MCP',
  'sidebar.newSession': 'نشست جدید', 'sidebar.sessions': 'نشست‌ها', 'sidebar.search': 'جستجوی نشست‌ها…', 'sidebar.settings': 'تنظیمات', 'sidebar.noSessions': 'هنوز نشستی نیست', 'sidebar.noResults': 'نتیجه‌ای نیست', 'sidebar.loading': 'در حال بارگذاری…',
  'input.placeholder': 'پیامی بنویسید یا / برای دستورها…', 'input.connectFirst': 'ابتدا یک ارائه‌دهنده را متصل کنید…', 'input.context': '+ زمینه', 'header.selectModel': 'انتخاب مدل', 'header.loadingSession': 'در حال بارگذاری نشست…', 'settings.backToChat': 'بازگشت به گفتگو',
}
const hi: Dict = {
  'welcome.subtitle': 'एआई कोडिंग सहायक', 'welcome.shortcuts': 'शॉर्टकट', 'welcome.send': 'संदेश भेजें', 'welcome.newline': 'नई पंक्ति', 'welcome.openSettings': 'सेटिंग्स खोलें', 'welcome.modes': 'मोड',
  'mode.build.desc': 'मॉडल का सीधा उत्तर, कोई टूल नहीं', 'mode.plan.desc': 'केवल पढ़ें: कोड को बिना संपादन के देखें', 'mode.agent.desc': 'ORION टूल का उपयोग करता है: पढ़ना, लिखना, bash, MCP',
  'sidebar.newSession': 'नया सत्र', 'sidebar.sessions': 'सत्र', 'sidebar.search': 'सत्र खोजें…', 'sidebar.settings': 'सेटिंग्स', 'sidebar.noSessions': 'अभी कोई सत्र नहीं', 'sidebar.noResults': 'कोई परिणाम नहीं', 'sidebar.loading': 'लोड हो रहा है…',
  'input.placeholder': 'संदेश लिखें या कमांड के लिए /…', 'input.connectFirst': 'पहले एक प्रदाता कनेक्ट करें…', 'input.context': '+ संदर्भ', 'header.selectModel': 'मॉडल चुनें', 'header.loadingSession': 'सत्र लोड हो रहा है…', 'settings.backToChat': 'चैट पर लौटें',
}
const id: Dict = {
  'welcome.subtitle': 'Asisten pemrograman AI', 'welcome.shortcuts': 'Pintasan', 'welcome.send': 'Kirim pesan', 'welcome.newline': 'Baris baru', 'welcome.openSettings': 'Buka pengaturan', 'welcome.modes': 'Mode',
  'mode.build.desc': 'Jawaban langsung model, tanpa alat', 'mode.plan.desc': 'Hanya-baca: jelajahi kode tanpa mengedit', 'mode.agent.desc': 'ORION memakai alat: baca, tulis, bash, MCP',
  'sidebar.newSession': 'Sesi baru', 'sidebar.sessions': 'Sesi', 'sidebar.search': 'Cari sesi…', 'sidebar.settings': 'Pengaturan', 'sidebar.noSessions': 'Belum ada sesi', 'sidebar.noResults': 'Tidak ada hasil', 'sidebar.loading': 'Memuat…',
  'input.placeholder': 'Ketik pesan atau / untuk perintah…', 'input.connectFirst': 'Hubungkan penyedia dulu…', 'input.context': '+ Konteks', 'header.selectModel': 'Pilih model', 'header.loadingSession': 'Memuat sesi…', 'settings.backToChat': 'Kembali ke obrolan',
}
const vi: Dict = {
  'welcome.subtitle': 'Trợ lý lập trình AI', 'welcome.shortcuts': 'Phím tắt', 'welcome.send': 'Gửi tin nhắn', 'welcome.newline': 'Dòng mới', 'welcome.openSettings': 'Mở cài đặt', 'welcome.modes': 'Chế độ',
  'mode.build.desc': 'Phản hồi trực tiếp của mô hình, không công cụ', 'mode.plan.desc': 'Chỉ đọc: khám phá mã mà không chỉnh sửa', 'mode.agent.desc': 'ORION dùng công cụ: đọc, ghi, bash, MCP',
  'sidebar.newSession': 'Phiên mới', 'sidebar.sessions': 'Phiên', 'sidebar.search': 'Tìm phiên…', 'sidebar.settings': 'Cài đặt', 'sidebar.noSessions': 'Chưa có phiên nào', 'sidebar.noResults': 'Không có kết quả', 'sidebar.loading': 'Đang tải…',
  'input.placeholder': 'Nhập tin nhắn hoặc / để dùng lệnh…', 'input.connectFirst': 'Kết nối nhà cung cấp trước…', 'input.context': '+ Bối cảnh', 'header.selectModel': 'Chọn mô hình', 'header.loadingSession': 'Đang tải phiên…', 'settings.backToChat': 'Quay lại trò chuyện',
}
const th: Dict = {
  'welcome.subtitle': 'ผู้ช่วยเขียนโค้ด AI', 'welcome.shortcuts': 'ทางลัด', 'welcome.send': 'ส่งข้อความ', 'welcome.newline': 'ขึ้นบรรทัดใหม่', 'welcome.openSettings': 'เปิดการตั้งค่า', 'welcome.modes': 'โหมด',
  'mode.build.desc': 'การตอบกลับโดยตรงของโมเดล ไม่มีเครื่องมือ', 'mode.plan.desc': 'อ่านอย่างเดียว: สำรวจโค้ดโดยไม่แก้ไข', 'mode.agent.desc': 'ORION ใช้เครื่องมือ: อ่าน เขียน bash MCP',
  'sidebar.newSession': 'เซสชันใหม่', 'sidebar.sessions': 'เซสชัน', 'sidebar.search': 'ค้นหาเซสชัน…', 'sidebar.settings': 'การตั้งค่า', 'sidebar.noSessions': 'ยังไม่มีเซสชัน', 'sidebar.noResults': 'ไม่มีผลลัพธ์', 'sidebar.loading': 'กำลังโหลด…',
  'input.placeholder': 'พิมพ์ข้อความ หรือ / เพื่อใช้คำสั่ง…', 'input.connectFirst': 'เชื่อมต่อผู้ให้บริการก่อน…', 'input.context': '+ บริบท', 'header.selectModel': 'เลือกโมเดล', 'header.loadingSession': 'กำลังโหลดเซสชัน…', 'settings.backToChat': 'กลับไปที่แชท',
}
const cs: Dict = {
  'welcome.subtitle': 'AI asistent pro programování', 'welcome.shortcuts': 'Zkratky', 'welcome.send': 'Odeslat zprávu', 'welcome.newline': 'Nový řádek', 'welcome.openSettings': 'Otevřít nastavení', 'welcome.modes': 'Režimy',
  'mode.build.desc': 'Přímá odpověď modelu, bez nástrojů', 'mode.plan.desc': 'Pouze ke čtení: prozkoumat kód bez úprav', 'mode.agent.desc': 'ORION používá nástroje: čtení, zápis, bash, MCP',
  'sidebar.newSession': 'Nová relace', 'sidebar.sessions': 'Relace', 'sidebar.search': 'Hledat relace…', 'sidebar.settings': 'Nastavení', 'sidebar.noSessions': 'Zatím žádné relace', 'sidebar.noResults': 'Žádné výsledky', 'sidebar.loading': 'Načítání…',
  'input.placeholder': 'Napište zprávu nebo / pro příkazy…', 'input.connectFirst': 'Nejprve připojte poskytovatele…', 'input.context': '+ Kontext', 'header.selectModel': 'Vybrat model', 'header.loadingSession': 'Načítání relace…', 'settings.backToChat': 'Zpět do chatu',
}
const sv: Dict = {
  'welcome.subtitle': 'AI-kodningsassistent', 'welcome.shortcuts': 'Genvägar', 'welcome.send': 'Skicka meddelande', 'welcome.newline': 'Ny rad', 'welcome.openSettings': 'Öppna inställningar', 'welcome.modes': 'Lägen',
  'mode.build.desc': 'Direkt modellsvar, inga verktyg', 'mode.plan.desc': 'Skrivskyddat: utforska koden utan att redigera', 'mode.agent.desc': 'ORION använder verktyg: läs, skriv, bash, MCP',
  'sidebar.newSession': 'Ny session', 'sidebar.sessions': 'Sessioner', 'sidebar.search': 'Sök sessioner…', 'sidebar.settings': 'Inställningar', 'sidebar.noSessions': 'Inga sessioner än', 'sidebar.noResults': 'Inga resultat', 'sidebar.loading': 'Laddar…',
  'input.placeholder': 'Skriv ett meddelande eller / för kommandon…', 'input.connectFirst': 'Anslut en leverantör först…', 'input.context': '+ Kontext', 'header.selectModel': 'Välj modell', 'header.loadingSession': 'Laddar session…', 'settings.backToChat': 'Tillbaka till chatten',
}
const el: Dict = {
  'welcome.subtitle': 'Βοηθός προγραμματισμού AI', 'welcome.shortcuts': 'Συντομεύσεις', 'welcome.send': 'Αποστολή μηνύματος', 'welcome.newline': 'Νέα γραμμή', 'welcome.openSettings': 'Άνοιγμα ρυθμίσεων', 'welcome.modes': 'Λειτουργίες',
  'mode.build.desc': 'Άμεση απάντηση μοντέλου, χωρίς εργαλεία', 'mode.plan.desc': 'Μόνο ανάγνωση: εξερεύνηση κώδικα χωρίς επεξεργασία', 'mode.agent.desc': 'Το ORION χρησιμοποιεί εργαλεία: ανάγνωση, εγγραφή, bash, MCP',
  'sidebar.newSession': 'Νέα συνεδρία', 'sidebar.sessions': 'Συνεδρίες', 'sidebar.search': 'Αναζήτηση συνεδριών…', 'sidebar.settings': 'Ρυθμίσεις', 'sidebar.noSessions': 'Καμία συνεδρία ακόμη', 'sidebar.noResults': 'Κανένα αποτέλεσμα', 'sidebar.loading': 'Φόρτωση…',
  'input.placeholder': 'Γράψτε ένα μήνυμα ή / για εντολές…', 'input.connectFirst': 'Συνδέστε πρώτα έναν πάροχο…', 'input.context': '+ Πλαίσιο', 'header.selectModel': 'Επιλογή μοντέλου', 'header.loadingSession': 'Φόρτωση συνεδρίας…', 'settings.backToChat': 'Επιστροφή στη συνομιλία',
}
const ro: Dict = {
  'welcome.subtitle': 'Asistent de programare AI', 'welcome.shortcuts': 'Scurtături', 'welcome.send': 'Trimite mesajul', 'welcome.newline': 'Rând nou', 'welcome.openSettings': 'Deschide setările', 'welcome.modes': 'Moduri',
  'mode.build.desc': 'Răspuns direct al modelului, fără unelte', 'mode.plan.desc': 'Doar citire: explorează codul fără a-l edita', 'mode.agent.desc': 'ORION folosește unelte: citire, scriere, bash, MCP',
  'sidebar.newSession': 'Sesiune nouă', 'sidebar.sessions': 'Sesiuni', 'sidebar.search': 'Caută sesiuni…', 'sidebar.settings': 'Setări', 'sidebar.noSessions': 'Încă nicio sesiune', 'sidebar.noResults': 'Niciun rezultat', 'sidebar.loading': 'Se încarcă…',
  'input.placeholder': 'Scrie un mesaj sau / pentru comenzi…', 'input.connectFirst': 'Conectează mai întâi un furnizor…', 'input.context': '+ Context', 'header.selectModel': 'Alege modelul', 'header.loadingSession': 'Se încarcă sesiunea…', 'settings.backToChat': 'Înapoi la chat',
}
const hu: Dict = {
  'welcome.subtitle': 'MI programozási asszisztens', 'welcome.shortcuts': 'Billentyűparancsok', 'welcome.send': 'Üzenet küldése', 'welcome.newline': 'Új sor', 'welcome.openSettings': 'Beállítások megnyitása', 'welcome.modes': 'Módok',
  'mode.build.desc': 'Közvetlen modellválasz, eszközök nélkül', 'mode.plan.desc': 'Csak olvasható: kód felfedezése szerkesztés nélkül', 'mode.agent.desc': 'Az ORION eszközöket használ: olvasás, írás, bash, MCP',
  'sidebar.newSession': 'Új munkamenet', 'sidebar.sessions': 'Munkamenetek', 'sidebar.search': 'Munkamenetek keresése…', 'sidebar.settings': 'Beállítások', 'sidebar.noSessions': 'Még nincs munkamenet', 'sidebar.noResults': 'Nincs találat', 'sidebar.loading': 'Betöltés…',
  'input.placeholder': 'Írj üzenetet vagy / a parancsokhoz…', 'input.connectFirst': 'Először csatlakoztass egy szolgáltatót…', 'input.context': '+ Kontextus', 'header.selectModel': 'Modell kiválasztása', 'header.loadingSession': 'Munkamenet betöltése…', 'settings.backToChat': 'Vissza a csevegéshez',
}
const ca: Dict = {
  'welcome.subtitle': 'Assistent de programació amb IA', 'welcome.shortcuts': 'Dreceres', 'welcome.send': 'Envia el missatge', 'welcome.newline': 'Línia nova', 'welcome.openSettings': 'Obre la configuració', 'welcome.modes': 'Modes',
  'mode.build.desc': 'Resposta directa del model, sense eines', 'mode.plan.desc': 'Només lectura: explora el codi sense editar-lo', 'mode.agent.desc': 'ORION fa servir eines: llegir, escriure, bash, MCP',
  'sidebar.newSession': 'Sessió nova', 'sidebar.sessions': 'Sessions', 'sidebar.search': 'Cerca sessions…', 'sidebar.settings': 'Configuració', 'sidebar.noSessions': 'Encara no hi ha sessions', 'sidebar.noResults': 'Cap resultat', 'sidebar.loading': 'Carregant…',
  'input.placeholder': 'Escriu un missatge o / per a ordres…', 'input.connectFirst': 'Connecta primer un proveïdor…', 'input.context': '+ Context', 'header.selectModel': 'Tria el model', 'header.loadingSession': 'Carregant la sessió…', 'settings.backToChat': 'Torna al xat',
}
const ms: Dict = {
  'welcome.subtitle': 'Pembantu pengekodan AI', 'welcome.shortcuts': 'Pintasan', 'welcome.send': 'Hantar mesej', 'welcome.newline': 'Baris baharu', 'welcome.openSettings': 'Buka tetapan', 'welcome.modes': 'Mod',
  'mode.build.desc': 'Respons terus model, tiada alat', 'mode.plan.desc': 'Baca sahaja: terokai kod tanpa menyunting', 'mode.agent.desc': 'ORION guna alat: baca, tulis, bash, MCP',
  'sidebar.newSession': 'Sesi baharu', 'sidebar.sessions': 'Sesi', 'sidebar.search': 'Cari sesi…', 'sidebar.settings': 'Tetapan', 'sidebar.noSessions': 'Belum ada sesi', 'sidebar.noResults': 'Tiada keputusan', 'sidebar.loading': 'Memuatkan…',
  'input.placeholder': 'Taip mesej atau / untuk arahan…', 'input.connectFirst': 'Sambungkan pembekal dahulu…', 'input.context': '+ Konteks', 'header.selectModel': 'Pilih model', 'header.loadingSession': 'Memuatkan sesi…', 'settings.backToChat': 'Kembali ke sembang',
}
const da: Dict = {
  'welcome.subtitle': 'AI-kodningsassistent', 'welcome.shortcuts': 'Genveje', 'welcome.send': 'Send besked', 'welcome.newline': 'Ny linje', 'welcome.openSettings': 'Åbn indstillinger', 'welcome.modes': 'Tilstande',
  'mode.build.desc': 'Direkte modelsvar, ingen værktøjer', 'mode.plan.desc': 'Skrivebeskyttet: udforsk koden uden at redigere', 'mode.agent.desc': 'ORION bruger værktøjer: læs, skriv, bash, MCP',
  'sidebar.newSession': 'Ny session', 'sidebar.sessions': 'Sessioner', 'sidebar.search': 'Søg sessioner…', 'sidebar.settings': 'Indstillinger', 'sidebar.noSessions': 'Ingen sessioner endnu', 'sidebar.noResults': 'Ingen resultater', 'sidebar.loading': 'Indlæser…',
  'input.placeholder': 'Skriv en besked eller / for kommandoer…', 'input.connectFirst': 'Forbind en udbyder først…', 'input.context': '+ Kontekst', 'header.selectModel': 'Vælg model', 'header.loadingSession': 'Indlæser session…', 'settings.backToChat': 'Tilbage til chat',
}
const fi: Dict = {
  'welcome.subtitle': 'Tekoälykoodausavustaja', 'welcome.shortcuts': 'Pikanäppäimet', 'welcome.send': 'Lähetä viesti', 'welcome.newline': 'Uusi rivi', 'welcome.openSettings': 'Avaa asetukset', 'welcome.modes': 'Tilat',
  'mode.build.desc': 'Mallin suora vastaus, ei työkaluja', 'mode.plan.desc': 'Vain luku: tutki koodia muokkaamatta', 'mode.agent.desc': 'ORION käyttää työkaluja: luku, kirjoitus, bash, MCP',
  'sidebar.newSession': 'Uusi istunto', 'sidebar.sessions': 'Istunnot', 'sidebar.search': 'Hae istuntoja…', 'sidebar.settings': 'Asetukset', 'sidebar.noSessions': 'Ei vielä istuntoja', 'sidebar.noResults': 'Ei tuloksia', 'sidebar.loading': 'Ladataan…',
  'input.placeholder': 'Kirjoita viesti tai / komentoja varten…', 'input.connectFirst': 'Yhdistä ensin palveluntarjoaja…', 'input.context': '+ Konteksti', 'header.selectModel': 'Valitse malli', 'header.loadingSession': 'Ladataan istuntoa…', 'settings.backToChat': 'Takaisin keskusteluun',
}
const zhTW: Dict = {
  'welcome.subtitle': 'AI 程式設計助理', 'welcome.shortcuts': '快速鍵', 'welcome.send': '傳送訊息', 'welcome.newline': '換行', 'welcome.openSettings': '開啟設定', 'welcome.modes': '模式',
  'mode.build.desc': '模型直接回覆，不使用工具', 'mode.plan.desc': '唯讀：瀏覽程式碼而不修改', 'mode.agent.desc': 'ORION 使用工具：讀取、寫入、bash、MCP',
  'sidebar.newSession': '新工作階段', 'sidebar.sessions': '工作階段', 'sidebar.search': '搜尋工作階段…', 'sidebar.settings': '設定', 'sidebar.noSessions': '尚無工作階段', 'sidebar.noResults': '沒有結果', 'sidebar.loading': '載入中…',
  'input.placeholder': '輸入訊息，或輸入 / 使用指令…', 'input.connectFirst': '請先連接服務商…', 'input.context': '+ 內容', 'header.selectModel': '選擇模型', 'header.loadingSession': '正在載入工作階段…', 'settings.backToChat': '返回聊天',
}

const TRANSLATIONS: Record<Lang, Dict> = {
  en, es, pt, fr, de, it, zh, ja, ko, ru,
  nl, pl, tr, uk, ar, he, fa, hi, id, vi, th, cs, sv, el, ro, hu, ca, ms, da, fi, zhTW,
}

// Right-to-left languages.
export const RTL_LANGS: Lang[] = ['ar', 'he', 'fa']

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
  // Set document direction for right-to-left languages.
  useEffect(() => {
    document.documentElement.dir = RTL_LANGS.includes(lang) ? 'rtl' : 'ltr'
  }, [lang])
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
