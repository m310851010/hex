//! The panes' newest translated strings — the Modes pane lab, the Modes
//! editor's websites and vocabulary, the vocabulary boost, the screenshots
//! folder, the meeting indicator corner, and the capsule flow — kept beside
//! [`super::TRANSLATIONS`] so the main table stays under the source cap.

/// Columns: Polish, Chinese (Simplified), Japanese, German, Spanish, Ukrainian.
#[rustfmt::skip]
pub(super) const TRANSLATIONS: &[(&str, [&str; 6])] = &[
    ("Modes pane", ["Panel trybów", "模式面板", "モードパネル", "Modi-Bereich", "Panel de modos", "Панель режимів"]),
    ("Settings' Modes pane joins the sidebar, where replacements, transformations, and AI rewriting are edited directly", ["Panel trybów w Ustawieniach dołącza do paska bocznego, gdzie poprawki, przekształcenia i przepisywanie AI edytuje się bezpośrednio", "设置中的模式面板加入侧边栏，可直接编辑替换、变换和 AI 改写", "設定のモードパネルがサイドバーに現れ、置換・変換・AI書き換えを直接編集できます", "Der Modi-Bereich der Einstellungen kommt in die Seitenleiste, wo Ersetzungen, Transformationen und KI-Umschreiben direkt bearbeitet werden", "El panel de modos de Ajustes se une a la barra lateral, donde se editan directamente reemplazos, transformaciones y reescritura con IA", "Панель режимів у Налаштуваннях з'являється на бічній панелі, де редагуються заміни, перетворення та AI-переписування"]),
    ("Bias", ["Wzmocnienie", "偏置", "バイアス", "Verstärkung", "Sesgo", "Підсилення"]),
    ("Change folder", ["Zmień folder", "更改文件夹", "フォルダを変更", "Ordner ändern", "Cambiar carpeta", "Змінити папку"]),
    ("Screenshots folder", ["Folder zrzutów", "截图文件夹", "スクリーンショットフォルダ", "Screenshot-Ordner", "Carpeta de capturas", "Папка знімків"]),
    ("Indicator corner", ["Narożnik wskaźnika", "指示器角落", "インジケーターのコーナー", "Ecke des Indikators", "Esquina del indicador", "Кут індикатора"]),
    ("Top right", ["Prawy górny", "右上", "右上", "Oben rechts", "Superior derecha", "Праворуч зверху"]),
    ("Top left", ["Lewy górny", "左上", "左上", "Oben links", "Superior izquierda", "Ліворуч зверху"]),
    ("Bottom right", ["Prawy dolny", "右下", "右下", "Unten rechts", "Inferior derecha", "Праворуч знизу"]),
    ("Bottom left", ["Lewy dolny", "左下", "左下", "Unten links", "Inferior izquierda", "Ліворуч знизу"]),
    ("Where the recording pill sits; dragging it there saves the same setting", ["Gdzie stoi pigułka nagrywania; przeciągnięcie jej tam zapisuje to samo ustawienie", "录制胶囊所在的位置；拖到某处即保存为同一设置", "録音ピルの位置。ドラッグで移動すると同じ設定が保存されます", "Wo die Aufnahme-Pille sitzt; sie dorthin zu ziehen speichert dieselbe Einstellung", "Dónde se sitúa la píldora de grabación; arrastrarla allí guarda el mismo ajuste", "Де стоїть пігулка запису; перетягування туди зберігає те саме налаштування"]),
    ("Capsule flow", ["Przepływ kapsuły", "胶囊流程", "カプセルの流れ", "Kapsel-Ablauf", "Flujo de cápsula", "Потік пігулки"]),
    ("Capsule", ["Kapsuła", "胶囊", "カプセル", "Kapsel", "Cápsula", "Пігулка"]),
    ("How the recording capsule offers its destinations", ["Jak kapsuła nagrywania proponuje cele", "录制胶囊如何提供发送目标", "録音カプセルが送信先を提示する方法", "Wie die Aufnahme-Kapsel ihre Ziele anbietet", "Cómo la cápsula de grabación ofrece sus destinos", "Як пігулка запису пропонує цілі"]),
    ("List", ["Lista", "列表", "リスト", "Liste", "Lista", "Список"]),
    ("Toast", ["Toast", "轻提示", "トースト", "Toast", "Toast", "Тост"]),
    ("Cursor", ["Kursor", "指针处", "カーソル", "Cursor", "Cursor", "Курсор"]),
    ("Notch", ["Wycięcie", "刘海", "ノッチ", "Notch", "Notch", "Виріз"]),
    ("Add website", ["Dodaj witrynę", "添加网站", "サイトを追加", "Website hinzufügen", "Añadir sitio", "Додати сайт"]),
    ("No websites in this mode.", ["Brak witryn w tym trybie.", "此模式没有网站。", "このモードにサイトはありません。", "Keine Websites in diesem Modus.", "No hay sitios en este modo.", "У цьому режимі немає сайтів."]),
    ("Website host (e.g. github.com)", ["Host witryny (np. github.com)", "网站域名（如 github.com）", "サイトのホスト（例: github.com）", "Website-Host (z. B. github.com)", "Host del sitio (p. ej. github.com)", "Хост сайту (наприклад github.com)"]),
    ("Websites", ["Witryny", "网站", "サイト", "Websites", "Sitios", "Сайти"]),
    ("No vocabulary in this mode; the shared list always applies.", ["Brak słownictwa w tym trybie; wspólna lista zawsze obowiązuje.", "此模式没有词表；共享词表始终生效。", "このモードの語彙はありません。共通リストは常に適用されます。", "Kein Vokabular in diesem Modus; die gemeinsame Liste gilt immer.", "Sin vocabulario en este modo; la lista compartida siempre se aplica.", "У цьому режимі немає словника; спільний список діє завжди."]),
    ("OpenAI compatible", ["Zgodny z OpenAI", "OpenAI 兼容", "OpenAI 互換", "OpenAI-kompatibel", "Compatible con OpenAI", "Сумісний з OpenAI"]),
    ("Anthropic", ["Anthropic", "Anthropic", "Anthropic", "Anthropic", "Anthropic", "Anthropic"]),
    ("AI rewriting", ["Przepisywanie AI", "AI 改写", "AI 書き換え", "KI-Umschreiben", "Reescritura con IA", "AI-переписування"]),
    ("Provider", ["Dostawca", "提供方", "プロバイダー", "Anbieter", "Proveedor", "Провайдер"]),
    ("Base URL", ["Adres bazowy", "Base URL", "ベース URL", "Basis-URL", "URL base", "Базовий URL"]),
    ("API key", ["Klucz API", "API Key", "API キー", "API-Schlüssel", "Clave API", "Ключ API"]),
    ("Model id", ["Identyfikator modelu", "模型 ID", "モデル ID", "Modell-ID", "Id del modelo", "ID моделі"]),
    ("Fetch models", ["Pobierz modele", "拉取模型", "モデルを取得", "Modelle abrufen", "Obtener modelos", "Отримати моделі"]),
    ("Test connection", ["Testuj połączenie", "测试连接", "接続をテスト", "Verbindung testen", "Probar conexión", "Перевірити з'єднання"]),
    ("Connection works", ["Połączenie działa", "连接正常", "接続成功", "Verbindung funktioniert", "La conexión funciona", "З'єднання працює"]),
    ("Stored in settings.json as plain text", ["Przechowywany w settings.json jako zwykły tekst", "以明文存储在 settings.json 中", "settings.json に平文で保存されます", "Als Klartext in settings.json gespeichert", "Guardado como texto plano en settings.json", "Зберігається у settings.json відкритим текстом"]),
    ("OpenCode service, or a direct endpoint you configure", ["Usługa OpenCode albo własny endpoint", "OpenCode 服务，或你配置的直连端点", "OpenCode サービス、または設定した直接エンドポイント", "OpenCode-Dienst oder ein selbst konfigurierter Endpunkt", "Servicio OpenCode o un endpoint directo que configures", "Служба OpenCode або власна кінцева точка"]),
    ("Describe how the model should rewrite your dictation…", ["Opisz, jak model ma przepisywać dyktowanie…", "描述模型应如何改写你的听写…", "音声入力をどう書き換えるか記述…", "Beschreibe, wie das Modell dein Diktat umschreiben soll…", "Describe cómo el modelo debe reescribir tu dictado…", "Опишіть, як модель має переписувати диктування…"]),
    ("Search models", ["Szukaj modeli", "搜索模型", "モデルを検索", "Modelle suchen", "Buscar modelos", "Пошук моделей"]),
];
