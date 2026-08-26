// Long-form Settings prose (Help FAQ, Security, licenses), one `LongForm` per
// language, in the same order as the `Strings` tables in i18n_data.rs. These are
// machine-assisted translations; a native review pass is recommended before a
// wide release. Product names, file extensions, paths and SPDX license ids
// (pass, passage, .gpg-id, otpauth://, age, rpgp, OpenPGP, TOTP, MIT, Apache-2.0)
// are left verbatim across every language on purpose.

// -------------------------------------------------------------------- en ---
pub const EN_LONG: LongForm = LongForm {
    faq: &[
        (
            "What are pass and passage?",
            "pass stores each password as a small OpenPGP-encrypted file; passage is the same idea with age encryption. PassPony reads and writes both, and the files stay compatible with the pass and passage command-line tools and with PassPony on your phone.",
        ),
        (
            "What is a recipient?",
            "The key an entry is encrypted to. For pass it is the fingerprint in the store's .gpg-id; for passage it is an age recipient. You can encrypt to a recipient with only their public key.",
        ),
        (
            "Why can I browse but not open an entry?",
            "Browsing and search never decrypt anything. Opening an entry needs the matching key or identity loaded (and, for a passphrase-locked pass key, the passphrase entered under Keys).",
        ),
        (
            "What is the TOTP code on an entry?",
            "If an entry has an otpauth:// line, PassPony shows the current two-factor code with a countdown ring, the same code your authenticator app would.",
        ),
        (
            "Is git sync a backup of my keys?",
            "No. Sync pushes only the encrypted entries. Your keys never leave this machine unless you export them. Back up your keys separately (Settings › Back up your keys).",
        ),
        (
            "What happens if I lose my key?",
            "Entries encrypted to it can no longer be decrypted, by anyone. There is no recovery. Keep a backup of your key somewhere safe.",
        ),
        (
            "Can these files be opened without PassPony?",
            "Yes. They are standard pass (.gpg) and passage (.age) files. The pass and passage CLIs, and other OpenPGP/age tools, open them with the right key.",
        ),
    ],
    security: &[
        (
            "Everything stays on this computer",
            "PassPony makes no network requests except git sync, to the remote you configure. Encryption, decryption, and your keys never leave the machine otherwise. The links on this screen open your browser; the app itself does not connect.",
        ),
        (
            "The formats and their algorithms",
            "pass uses OpenPGP (via rpgp); passage uses age. PassPony writes standard files for both, so other pass/age tools and the phone apps read them.",
        ),
        (
            "Your keys at rest",
            "Keys and age identities live in your config folder, readable by you alone on Unix. A pass secret key can be passphrase-protected so the file itself is encrypted.",
        ),
        (
            "Plaintext in memory",
            "Decrypted entries are held in zeroizing buffers and cleared when you close them; copied secrets clear from the clipboard after 45 seconds.",
        ),
    ],
    security_not_title: "What PassPony does not do",
    security_not: &[
        "Git sync backs up your entries, not your keys. Back up your keys yourself.",
        "It cannot recover a key you have lost, nor the entries encrypted to it.",
        "It sends no analytics or telemetry, ever.",
        "It cannot protect files after they leave PassPony, or defend a computer already compromised.",
    ],
    crates: &[
        ("pgp (rpgp) — the OpenPGP engine", "MIT or Apache-2.0"),
        ("age, zeroize — the age engine", "MIT or Apache-2.0"),
        ("pass-core (PassPonyCore) — store, git, TOTP", "Apache-2.0"),
        ("egui, eframe — the user interface", "MIT or Apache-2.0"),
        ("git2, arboard, rfd, directories", "MIT or Apache-2.0"),
    ],
};

// -------------------------------------------------------------------- es ---
pub const ES_LONG: LongForm = LongForm {
    faq: &[
        (
            "¿Qué son pass y passage?",
            "pass guarda cada contraseña como un pequeño archivo cifrado con OpenPGP; passage es la misma idea con cifrado age. PassPony lee y escribe ambos, y los archivos siguen siendo compatibles con las herramientas de línea de comandos pass y passage y con PassPony en tu teléfono.",
        ),
        (
            "¿Qué es un destinatario?",
            "La clave con la que se cifra una entrada. En pass es la huella del .gpg-id del almacén; en passage es un destinatario age. Puedes cifrar para un destinatario teniendo solo su clave pública.",
        ),
        (
            "¿Por qué puedo explorar pero no abrir una entrada?",
            "Explorar y buscar nunca descifran nada. Abrir una entrada requiere tener cargada la clave o identidad correspondiente (y, para una clave pass protegida por frase, la frase introducida en Claves).",
        ),
        (
            "¿Qué es el código TOTP de una entrada?",
            "Si una entrada tiene una línea otpauth://, PassPony muestra el código de doble factor actual con un anillo de cuenta atrás, el mismo que mostraría tu aplicación de autenticación.",
        ),
        (
            "¿La sincronización con git respalda mis claves?",
            "No. La sincronización solo sube las entradas cifradas. Tus claves nunca salen de este equipo salvo que las exportes. Respalda tus claves por separado (Ajustes › Copia de seguridad de tus claves).",
        ),
        (
            "¿Qué pasa si pierdo mi clave?",
            "Las entradas cifradas con ella ya no podrán descifrarse, por nadie. No hay recuperación. Guarda una copia de tu clave en un lugar seguro.",
        ),
        (
            "¿Se pueden abrir estos archivos sin PassPony?",
            "Sí. Son archivos estándar de pass (.gpg) y passage (.age). Las herramientas pass y passage, y otras de OpenPGP/age, los abren con la clave correcta.",
        ),
    ],
    security: &[
        (
            "Todo permanece en este equipo",
            "PassPony no hace ninguna petición de red salvo la sincronización con git, hacia el remoto que configures. El cifrado, el descifrado y tus claves no salen de la máquina de ningún otro modo. Los enlaces de esta pantalla abren tu navegador; la aplicación en sí no se conecta.",
        ),
        (
            "Los formatos y sus algoritmos",
            "pass usa OpenPGP (mediante rpgp); passage usa age. PassPony escribe archivos estándar para ambos, así que otras herramientas pass/age y las apps del teléfono los leen.",
        ),
        (
            "Tus claves en reposo",
            "Las claves y las identidades age viven en tu carpeta de configuración, legibles solo por ti en Unix. Una clave secreta pass puede protegerse con frase de contraseña para que el propio archivo quede cifrado.",
        ),
        (
            "Texto sin cifrar en memoria",
            "Las entradas descifradas se mantienen en búferes que se ponen a cero y se borran al cerrarlas; los secretos copiados se borran del portapapeles a los 45 segundos.",
        ),
    ],
    security_not_title: "Lo que PassPony no hace",
    security_not: &[
        "La sincronización con git respalda tus entradas, no tus claves. Respalda tus claves tú mismo.",
        "No puede recuperar una clave que hayas perdido, ni las entradas cifradas con ella.",
        "Nunca envía analíticas ni telemetría.",
        "No puede proteger los archivos una vez salen de PassPony, ni defender un equipo ya comprometido.",
    ],
    crates: &[
        ("pgp (rpgp) — el motor OpenPGP", "MIT o Apache-2.0"),
        ("age, zeroize — el motor age", "MIT o Apache-2.0"),
        ("pass-core (PassPonyCore) — almacén, git, TOTP", "Apache-2.0"),
        ("egui, eframe — la interfaz de usuario", "MIT o Apache-2.0"),
        ("git2, arboard, rfd, directories", "MIT o Apache-2.0"),
    ],
};

// -------------------------------------------------------------------- fr ---
pub const FR_LONG: LongForm = LongForm {
    faq: &[
        (
            "Qu'est-ce que pass et passage ?",
            "pass stocke chaque mot de passe dans un petit fichier chiffré avec OpenPGP ; passage suit la même idée avec le chiffrement age. PassPony lit et écrit les deux, et les fichiers restent compatibles avec les outils en ligne de commande pass et passage ainsi qu'avec PassPony sur votre téléphone.",
        ),
        (
            "Qu'est-ce qu'un destinataire ?",
            "La clé avec laquelle une entrée est chiffrée. Pour pass, c'est l'empreinte du .gpg-id du magasin ; pour passage, c'est un destinataire age. Vous pouvez chiffrer pour un destinataire en n'ayant que sa clé publique.",
        ),
        (
            "Pourquoi puis-je parcourir mais pas ouvrir une entrée ?",
            "Parcourir et rechercher ne déchiffrent jamais rien. Ouvrir une entrée nécessite que la clé ou l'identité correspondante soit chargée (et, pour une clé pass protégée par phrase secrète, la phrase saisie dans Clés).",
        ),
        (
            "Qu'est-ce que le code TOTP d'une entrée ?",
            "Si une entrée comporte une ligne otpauth://, PassPony affiche le code à deux facteurs actuel avec un anneau de compte à rebours, le même que votre application d'authentification.",
        ),
        (
            "La synchronisation git sauvegarde-t-elle mes clés ?",
            "Non. La synchronisation n'envoie que les entrées chiffrées. Vos clés ne quittent jamais cette machine sauf si vous les exportez. Sauvegardez vos clés séparément (Réglages › Sauvegarder vos clés).",
        ),
        (
            "Que se passe-t-il si je perds ma clé ?",
            "Les entrées chiffrées avec elle ne peuvent plus être déchiffrées, par personne. Il n'y a pas de récupération. Conservez une sauvegarde de votre clé en lieu sûr.",
        ),
        (
            "Ces fichiers peuvent-ils s'ouvrir sans PassPony ?",
            "Oui. Ce sont des fichiers pass (.gpg) et passage (.age) standard. Les outils pass et passage, et d'autres outils OpenPGP/age, les ouvrent avec la bonne clé.",
        ),
    ],
    security: &[
        (
            "Tout reste sur cet ordinateur",
            "PassPony ne fait aucune requête réseau hormis la synchronisation git, vers le dépôt distant que vous configurez. Le chiffrement, le déchiffrement et vos clés ne quittent jamais la machine autrement. Les liens de cet écran ouvrent votre navigateur ; l'application elle-même ne se connecte pas.",
        ),
        (
            "Les formats et leurs algorithmes",
            "pass utilise OpenPGP (via rpgp) ; passage utilise age. PassPony écrit des fichiers standard pour les deux, de sorte que d'autres outils pass/age et les applications mobiles les lisent.",
        ),
        (
            "Vos clés au repos",
            "Les clés et les identités age résident dans votre dossier de configuration, lisibles par vous seul sous Unix. Une clé secrète pass peut être protégée par une phrase secrète afin que le fichier lui-même soit chiffré.",
        ),
        (
            "Le texte en clair en mémoire",
            "Les entrées déchiffrées sont conservées dans des tampons remis à zéro et effacées quand vous les fermez ; les secrets copiés s'effacent du presse-papiers après 45 secondes.",
        ),
    ],
    security_not_title: "Ce que PassPony ne fait pas",
    security_not: &[
        "La synchronisation git sauvegarde vos entrées, pas vos clés. Sauvegardez vos clés vous-même.",
        "Elle ne peut pas récupérer une clé que vous avez perdue, ni les entrées chiffrées avec elle.",
        "Elle n'envoie jamais d'analyses ni de télémétrie.",
        "Elle ne peut pas protéger les fichiers une fois qu'ils ont quitté PassPony, ni défendre un ordinateur déjà compromis.",
    ],
    crates: &[
        ("pgp (rpgp) — le moteur OpenPGP", "MIT ou Apache-2.0"),
        ("age, zeroize — le moteur age", "MIT ou Apache-2.0"),
        ("pass-core (PassPonyCore) — magasin, git, TOTP", "Apache-2.0"),
        ("egui, eframe — l'interface utilisateur", "MIT ou Apache-2.0"),
        ("git2, arboard, rfd, directories", "MIT ou Apache-2.0"),
    ],
};

// -------------------------------------------------------------------- de ---
pub const DE_LONG: LongForm = LongForm {
    faq: &[
        (
            "Was sind pass und passage?",
            "pass speichert jedes Passwort als kleine, mit OpenPGP verschlüsselte Datei; passage ist dieselbe Idee mit age-Verschlüsselung. PassPony liest und schreibt beide, und die Dateien bleiben mit den Kommandozeilenwerkzeugen pass und passage sowie mit PassPony auf deinem Telefon kompatibel.",
        ),
        (
            "Was ist ein Empfänger?",
            "Der Schlüssel, für den ein Eintrag verschlüsselt wird. Bei pass ist es der Fingerabdruck in der .gpg-id des Speichers; bei passage ein age-Empfänger. Du kannst für einen Empfänger verschlüsseln, wenn du nur dessen öffentlichen Schlüssel hast.",
        ),
        (
            "Warum kann ich blättern, aber keinen Eintrag öffnen?",
            "Blättern und Suchen entschlüsseln nie etwas. Zum Öffnen eines Eintrags muss der passende Schlüssel oder die passende Identität geladen sein (und bei einem passphrasengeschützten pass-Schlüssel die unter Schlüssel eingegebene Passphrase).",
        ),
        (
            "Was ist der TOTP-Code an einem Eintrag?",
            "Hat ein Eintrag eine otpauth://-Zeile, zeigt PassPony den aktuellen Zwei-Faktor-Code mit einem Countdown-Ring an – denselben, den deine Authenticator-App zeigen würde.",
        ),
        (
            "Ist die git-Synchronisierung eine Sicherung meiner Schlüssel?",
            "Nein. Die Synchronisierung überträgt nur die verschlüsselten Einträge. Deine Schlüssel verlassen diesen Rechner nur, wenn du sie exportierst. Sichere deine Schlüssel gesondert (Einstellungen › Schlüssel sichern).",
        ),
        (
            "Was passiert, wenn ich meinen Schlüssel verliere?",
            "Damit verschlüsselte Einträge lassen sich von niemandem mehr entschlüsseln. Es gibt keine Wiederherstellung. Bewahre eine Sicherung deines Schlüssels an einem sicheren Ort auf.",
        ),
        (
            "Lassen sich diese Dateien ohne PassPony öffnen?",
            "Ja. Es sind standardmäßige pass- (.gpg) und passage- (.age) Dateien. Die pass- und passage-Kommandozeilenwerkzeuge und andere OpenPGP/age-Werkzeuge öffnen sie mit dem richtigen Schlüssel.",
        ),
    ],
    security: &[
        (
            "Alles bleibt auf diesem Computer",
            "PassPony stellt außer der git-Synchronisierung zum von dir konfigurierten Remote keine Netzwerkanfragen. Verschlüsselung, Entschlüsselung und deine Schlüssel verlassen den Rechner sonst nie. Die Links auf diesem Bildschirm öffnen deinen Browser; die App selbst stellt keine Verbindung her.",
        ),
        (
            "Die Formate und ihre Algorithmen",
            "pass verwendet OpenPGP (über rpgp); passage verwendet age. PassPony schreibt für beide Standarddateien, sodass andere pass/age-Werkzeuge und die Telefon-Apps sie lesen.",
        ),
        (
            "Deine Schlüssel im Ruhezustand",
            "Schlüssel und age-Identitäten liegen in deinem Konfigurationsordner, unter Unix nur für dich lesbar. Ein geheimer pass-Schlüssel kann mit einer Passphrase geschützt werden, sodass die Datei selbst verschlüsselt ist.",
        ),
        (
            "Klartext im Speicher",
            "Entschlüsselte Einträge werden in nullenden Puffern gehalten und beim Schließen gelöscht; kopierte Geheimnisse werden nach 45 Sekunden aus der Zwischenablage entfernt.",
        ),
    ],
    security_not_title: "Was PassPony nicht tut",
    security_not: &[
        "Die git-Synchronisierung sichert deine Einträge, nicht deine Schlüssel. Sichere deine Schlüssel selbst.",
        "Sie kann einen verlorenen Schlüssel nicht wiederherstellen, ebenso wenig die damit verschlüsselten Einträge.",
        "Sie sendet niemals Analysen oder Telemetrie.",
        "Sie kann Dateien nach dem Verlassen von PassPony nicht schützen und einen bereits kompromittierten Rechner nicht verteidigen.",
    ],
    crates: &[
        ("pgp (rpgp) — die OpenPGP-Engine", "MIT oder Apache-2.0"),
        ("age, zeroize — die age-Engine", "MIT oder Apache-2.0"),
        ("pass-core (PassPonyCore) — Speicher, git, TOTP", "Apache-2.0"),
        ("egui, eframe — die Benutzeroberfläche", "MIT oder Apache-2.0"),
        ("git2, arboard, rfd, directories", "MIT oder Apache-2.0"),
    ],
};

// -------------------------------------------------------------------- zh ---
pub const ZH_LONG: LongForm = LongForm {
    faq: &[
        (
            "什么是 pass 和 passage？",
            "pass 将每个密码保存为一个用 OpenPGP 加密的小文件；passage 是同样的思路，改用 age 加密。PassPony 都能读写，且这些文件与 pass、passage 命令行工具以及手机上的 PassPony 保持兼容。",
        ),
        (
            "什么是收件人？",
            "条目被加密给的密钥。对 pass 而言，它是存储库 .gpg-id 中的指纹；对 passage 而言，它是一个 age 收件人。只要有对方的公钥，你就能为该收件人加密。",
        ),
        (
            "为什么我能浏览却打不开某个条目？",
            "浏览和搜索从不解密任何内容。打开条目需要已加载相应的密钥或身份（对于用口令锁定的 pass 密钥，还需在“密钥”中输入口令）。",
        ),
        (
            "条目上的 TOTP 验证码是什么？",
            "如果条目里有一行 otpauth://，PassPony 会带倒计时圆环显示当前的双因素验证码，与你的验证器应用显示的一致。",
        ),
        (
            "git 同步会备份我的密钥吗？",
            "不会。同步只上传已加密的条目。除非你导出，否则密钥绝不会离开这台电脑。请单独备份你的密钥（设置 › 备份你的密钥）。",
        ),
        (
            "如果我丢失了密钥会怎样？",
            "用它加密的条目将无法再被任何人解密，且无法恢复。请把密钥的备份保存在安全的地方。",
        ),
        (
            "没有 PassPony 也能打开这些文件吗？",
            "可以。它们是标准的 pass（.gpg）和 passage（.age）文件。pass、passage 命令行工具以及其他 OpenPGP/age 工具，用正确的密钥即可打开。",
        ),
    ],
    security: &[
        (
            "一切都留在这台电脑上",
            "除了向你配置的远端进行 git 同步外，PassPony 不发起任何网络请求。加密、解密和你的密钥不会以其他方式离开本机。本页的链接会打开你的浏览器；应用本身不会联网。",
        ),
        (
            "格式及其算法",
            "pass 使用 OpenPGP（通过 rpgp）；passage 使用 age。PassPony 对两者都写入标准文件，因此其他 pass/age 工具和手机应用都能读取。",
        ),
        (
            "静态存储的密钥",
            "密钥和 age 身份保存在你的配置文件夹中，在 Unix 上仅你自己可读。pass 私钥可用口令保护，使文件本身处于加密状态。",
        ),
        (
            "内存中的明文",
            "已解密的条目保存在会清零的缓冲区中，关闭时即清除；复制的机密在 45 秒后从剪贴板清除。",
        ),
    ],
    security_not_title: "PassPony 不会做的事",
    security_not: &[
        "git 同步备份的是你的条目，而非密钥。请自行备份密钥。",
        "它无法找回你已丢失的密钥，也无法恢复用它加密的条目。",
        "它绝不发送任何分析或遥测数据。",
        "文件离开 PassPony 后它无法再保护，也无法防御已被入侵的电脑。",
    ],
    crates: &[
        ("pgp (rpgp) — OpenPGP 引擎", "MIT 或 Apache-2.0"),
        ("age, zeroize — age 引擎", "MIT 或 Apache-2.0"),
        ("pass-core (PassPonyCore) — 存储、git、TOTP", "Apache-2.0"),
        ("egui, eframe — 用户界面", "MIT 或 Apache-2.0"),
        ("git2, arboard, rfd, directories", "MIT 或 Apache-2.0"),
    ],
};

// -------------------------------------------------------------------- pt ---
pub const PT_LONG: LongForm = LongForm {
    faq: &[
        (
            "O que são pass e passage?",
            "O pass guarda cada senha como um pequeno arquivo criptografado com OpenPGP; o passage é a mesma ideia com criptografia age. O PassPony lê e grava ambos, e os arquivos continuam compatíveis com as ferramentas de linha de comando pass e passage e com o PassPony no seu celular.",
        ),
        (
            "O que é um destinatário?",
            "A chave para a qual uma entrada é criptografada. No pass é a impressão digital no .gpg-id do repositório; no passage é um destinatário age. Você pode criptografar para um destinatário tendo apenas a chave pública dele.",
        ),
        (
            "Por que consigo navegar, mas não abrir uma entrada?",
            "Navegar e buscar nunca descriptografam nada. Abrir uma entrada exige que a chave ou identidade correspondente esteja carregada (e, para uma chave pass protegida por frase-senha, a frase digitada em Chaves).",
        ),
        (
            "O que é o código TOTP em uma entrada?",
            "Se uma entrada tiver uma linha otpauth://, o PassPony mostra o código de dois fatores atual com um anel de contagem regressiva, o mesmo que o seu aplicativo autenticador mostraria.",
        ),
        (
            "A sincronização com git faz backup das minhas chaves?",
            "Não. A sincronização envia apenas as entradas criptografadas. Suas chaves nunca saem deste computador a menos que você as exporte. Faça backup das suas chaves separadamente (Configurações › Backup das suas chaves).",
        ),
        (
            "O que acontece se eu perder minha chave?",
            "As entradas criptografadas com ela não poderão mais ser descriptografadas, por ninguém. Não há recuperação. Guarde um backup da sua chave em local seguro.",
        ),
        (
            "Esses arquivos podem ser abertos sem o PassPony?",
            "Sim. São arquivos pass (.gpg) e passage (.age) padrão. As ferramentas pass e passage, e outras de OpenPGP/age, os abrem com a chave certa.",
        ),
    ],
    security: &[
        (
            "Tudo permanece neste computador",
            "O PassPony não faz nenhuma requisição de rede além da sincronização com git, para o remoto que você configurar. Criptografia, descriptografia e suas chaves não saem da máquina de nenhuma outra forma. Os links desta tela abrem o seu navegador; o aplicativo em si não se conecta.",
        ),
        (
            "Os formatos e seus algoritmos",
            "O pass usa OpenPGP (via rpgp); o passage usa age. O PassPony grava arquivos padrão para ambos, de modo que outras ferramentas pass/age e os aplicativos de celular os leiam.",
        ),
        (
            "Suas chaves em repouso",
            "As chaves e identidades age ficam na sua pasta de configuração, legíveis apenas por você no Unix. Uma chave secreta pass pode ser protegida por frase-senha, de modo que o próprio arquivo fique criptografado.",
        ),
        (
            "Texto sem criptografia na memória",
            "As entradas descriptografadas são mantidas em buffers que são zerados e apagadas ao fechá-las; segredos copiados são limpos da área de transferência após 45 segundos.",
        ),
    ],
    security_not_title: "O que o PassPony não faz",
    security_not: &[
        "A sincronização com git faz backup das suas entradas, não das suas chaves. Faça backup das suas chaves você mesmo.",
        "Não é capaz de recuperar uma chave perdida, nem as entradas criptografadas com ela.",
        "Nunca envia análises nem telemetria.",
        "Não é capaz de proteger arquivos depois que saem do PassPony, nem defender um computador já comprometido.",
    ],
    crates: &[
        ("pgp (rpgp) — o motor OpenPGP", "MIT ou Apache-2.0"),
        ("age, zeroize — o motor age", "MIT ou Apache-2.0"),
        ("pass-core (PassPonyCore) — repositório, git, TOTP", "Apache-2.0"),
        ("egui, eframe — a interface do usuário", "MIT ou Apache-2.0"),
        ("git2, arboard, rfd, directories", "MIT ou Apache-2.0"),
    ],
};

// -------------------------------------------------------------------- ru ---
pub const RU_LONG: LongForm = LongForm {
    faq: &[
        (
            "Что такое pass и passage?",
            "pass хранит каждый пароль в маленьком файле, зашифрованном OpenPGP; passage — та же идея с шифрованием age. PassPony читает и записывает оба формата, а файлы остаются совместимы с консольными утилитами pass и passage и с PassPony на телефоне.",
        ),
        (
            "Что такое получатель?",
            "Ключ, для которого зашифрована запись. Для pass это отпечаток из .gpg-id хранилища; для passage — получатель age. Можно шифровать для получателя, имея лишь его открытый ключ.",
        ),
        (
            "Почему я могу просматривать, но не открыть запись?",
            "Просмотр и поиск ничего не расшифровывают. Для открытия записи нужен загруженный соответствующий ключ или личность (а для pass-ключа под паролем — пароль, введённый в разделе «Ключи»).",
        ),
        (
            "Что такое код TOTP у записи?",
            "Если у записи есть строка otpauth://, PassPony показывает текущий код двухфакторной аутентификации с кольцом обратного отсчёта — тот же, что показало бы ваше приложение-аутентификатор.",
        ),
        (
            "Является ли синхронизация git резервной копией моих ключей?",
            "Нет. Синхронизация отправляет только зашифрованные записи. Ваши ключи не покидают этот компьютер, пока вы их не экспортируете. Делайте резервную копию ключей отдельно (Настройки › Резервная копия ключей).",
        ),
        (
            "Что произойдёт, если я потеряю ключ?",
            "Записи, зашифрованные им, больше никто не сможет расшифровать. Восстановления нет. Храните резервную копию ключа в надёжном месте.",
        ),
        (
            "Можно ли открыть эти файлы без PassPony?",
            "Да. Это обычные файлы pass (.gpg) и passage (.age). Утилиты pass и passage, а также другие инструменты OpenPGP/age открывают их правильным ключом.",
        ),
    ],
    security: &[
        (
            "Всё остаётся на этом компьютере",
            "PassPony не делает сетевых запросов, кроме синхронизации git с настроенным вами удалённым репозиторием. Шифрование, расшифровка и ваши ключи иначе не покидают компьютер. Ссылки на этом экране открывают браузер; само приложение не подключается к сети.",
        ),
        (
            "Форматы и их алгоритмы",
            "pass использует OpenPGP (через rpgp); passage использует age. PassPony записывает стандартные файлы для обоих, поэтому другие инструменты pass/age и мобильные приложения их читают.",
        ),
        (
            "Ваши ключи в состоянии покоя",
            "Ключи и личности age хранятся в вашей папке конфигурации, доступной для чтения только вам в Unix. Секретный ключ pass можно защитить паролем, чтобы сам файл был зашифрован.",
        ),
        (
            "Открытый текст в памяти",
            "Расшифрованные записи хранятся в обнуляемых буферах и очищаются при закрытии; скопированные секреты удаляются из буфера обмена через 45 секунд.",
        ),
    ],
    security_not_title: "Чего PassPony не делает",
    security_not: &[
        "Синхронизация git сохраняет ваши записи, а не ключи. Делайте резервную копию ключей самостоятельно.",
        "Он не может восстановить потерянный ключ, как и записи, зашифрованные им.",
        "Он никогда не отправляет аналитику или телеметрию.",
        "Он не может защитить файлы после того, как они покинули PassPony, и не защитит уже скомпрометированный компьютер.",
    ],
    crates: &[
        ("pgp (rpgp) — движок OpenPGP", "MIT или Apache-2.0"),
        ("age, zeroize — движок age", "MIT или Apache-2.0"),
        ("pass-core (PassPonyCore) — хранилище, git, TOTP", "Apache-2.0"),
        ("egui, eframe — пользовательский интерфейс", "MIT или Apache-2.0"),
        ("git2, arboard, rfd, directories", "MIT или Apache-2.0"),
    ],
};
