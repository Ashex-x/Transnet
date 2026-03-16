/**
 * Language service for managing application language state and translations.
 *
 * Provides centralized language management with localStorage persistence
 * and pub/sub notifications for language changes.
 */

type Language = 'en' | 'zh';
type TranslationKey = keyof typeof translations.en;

interface Translations {
  en: Record<string, string>;
  zh: Record<string, string>;
}

const translations: Translations = {
  en: {
    // Home page
    home: 'Home',
    about: 'About',
    setting: 'Setting',
    chooseIsland: 'Choose your island',
    welcome: 'Welcome to Island OS.',
    instructions: 'Right-click and drag to roll the sphere. Hover the named nodes to preview each island, then click to enter.',
    currentTheme: 'Current theme: Dark mode',
    backgroundPreserved: 'Background preserved',
    login: 'Login',
    signup: 'Signup',
    dark: 'Dark',

    // Home sphere
    metaland: 'Metaland',
    metalandDesc: 'Enter the living digital world',
    transnet: 'Transnet',
    transnetDesc: 'Open the AI translation bridge',

    // Auth pages
    welcomeBack: 'Welcome Back',
    loginOrRegister: 'Login or register to save your translation history',
    loginToSave: 'Login to save your translation history',
    registerToSave: 'Register to save your translation history',
    createAccount: 'Create Account',
    email: 'Email',
    password: 'Password',
    username: 'Username',
    emailPlaceholder: 'your@email.com',
    passwordPlaceholder: 'Enter password',
    passwordPlaceholderRegister: 'At least 8 chars with uppercase, lowercase and number',
    usernamePlaceholder: '3-50 chars, alphanumeric and underscore',
    processing: 'Processing...',
    loginSuccessful: 'Login successful',
    loginFailed: 'Login failed',
    registrationSuccessful: 'Registration successful, please log in',
    registrationFailed: 'Registration failed',
    dontHaveAccount: "Don't have an account?",
    alreadyHaveAccount: 'Already have an account?',
    passwordTooShort: 'Too short',
    passwordWeak: 'Weak',
    passwordFair: 'Fair',
    passwordStrong: 'Strong',
    passwordVeryStrong: 'Very Strong',

    // Settings page
    settings: 'Settings',
    settingsDescription: 'Customize your Transnet experience with these settings.',
    appearance: 'Appearance',
    darkMode: 'Dark Mode',
    darkModeFixed: 'Currently fixed to dark theme',
    languageSettings: 'Language',
    interfaceLanguage: 'Interface Language',
    languageSetViaHome: 'Set via home page language selector',
    configure: 'Configure',
    translationSettings: 'Translation',
    autoDetectInput: 'Auto-detect Input Type',
    smartDetectionEnabled: 'Smart detection enabled by default',
    saveToHistory: 'Save to History',
    autoSaveTranslations: 'Automatically save translations',
    privacy: 'Privacy',
    dataStorage: 'Data Storage',
    dataStoredSecurely: 'Your translations are stored securely',
    accountDeletion: 'Account Deletion',
    removeDataPermanently: 'Remove all your data permanently',
    requestDeletion: 'Request Deletion',
    settingsComingSoon: 'Settings Coming Soon:',
    settingsComingSoonDesc: 'More customization options will be available in future updates. Stay tuned for enhanced personalization features.',

    // About page
    aboutTransnet: 'About Transnet',
    aboutDescription: 'Transnet is an AI-powered intelligent translation platform designed to bridge language barriers across digital worlds. Our mission is to make seamless communication accessible to everyone.',
    multiLanguageSupport: 'Multi-Language Support',
    multiLanguageDesc: 'Translate between English, Spanish, French, German, Chinese, Japanese, and Korean with advanced AI models.',
    smartTranslation: 'Smart Translation',
    smartTranslationDesc: 'Automatically detects input types and provides context-aware translations for words, phrases, sentences, and full text.',
    translationHistory: 'Translation History',
    translationHistoryDesc: 'Keep track of all your translations with powerful search and filter capabilities to find past work quickly.',
    favorites: 'Favorites',
    favoritesDesc: 'Save your best translations with custom notes and enhanced word meanings for easy reference later.',
    builtWithAI: 'Built with cutting-edge AI technology',

    // Transnet page
    navTranslate: 'Translate',
    navHistory: 'History',
    navFavorites: 'Favorites',
    transnetSource: 'Source',
    transnetTarget: 'Target',
    transnetSourcePlaceholder: 'Enter text to translate... Press Ctrl+Enter to translate quickly.',
    transnetTargetPlaceholder: 'Translation will appear here...',
    transnetUploadImage: 'Click to upload an image',
    transnetOutputType: 'Output Type:',
    transnetInputType: 'Input Type:',
    transnetText: 'Text',
    transnetImage: 'Image',
    transnetTranslate: 'Translate',
    transnetTranslating: 'Translating...',
    transnetContactingBackend: 'Contacting Transnet backend...',
    transnetEnterTextBeforeTranslating: 'Enter text before translating.',
    transnetTranslationFailed: 'Translation failed.',
    transnetUnableToReach: 'Unable to reach Transnet backend.',
    transnetTranslatedAs: `Translated as {input_type} via {model}.`,
    transnetExtraOutput: 'Extra Output',
    transnetWordInformation: 'Word Information',
    transnetWord: 'Word',
    transnetPartOfSpeech: 'Part of Speech',
    transnetPhonetic: 'Phonetic',
    transnetSynonyms: 'Synonyms',
    transnetAntonyms: 'Antonyms',
    transnetExamples: 'Examples',
    transnetExplanation: 'Explanation',
    transnetMeaning: 'Meaning',
    transnetStory: 'Story',
    transnetWhenToUse: 'When to use',
    transnetHowToUse: 'How to use',
    transnetContext: 'Context',
    transnetRoot: 'Root',
    transnetStructure: 'Structure',
    transnetIdiomatic: 'Idiomatic',
    transnetYes: 'Yes',
    transnetRelationships: 'Relationships',
    transnetRelatedWords: 'Related Words',
    transnetByPartOfSpeech: 'By Part of Speech',
    transnetNouns: 'Nouns',
    transnetVerbs: 'Verbs',
    transnetAdjectives: 'Adjectives',
    transnetPhraseInformation: 'Phrase Information',
    transnetPhrase: 'Phrase',
    transnetRelatedPhrases: 'Related Phrases',
    transnetRelatedConcepts: 'Related Concepts',
    transnetSentenceInformation: 'Sentence Information',
    transnetTone: 'Tone',
    transnetUsage: 'Usage',
    transnetTranslationComparison: 'Translation Comparison',
    transnetOriginal: 'Original',
    transnetTranslation: 'Translation',

    // History page
    historyPageTitle: 'Translation History',
    historySource: 'Source',
    historyTarget: 'Target',
    historyType: 'Type',
    historyNoHistoryYet: 'No History Yet',
    historyStartTranslating: 'Start translating and your history will appear here',
    historyCopy: 'Copy',
    historyFavorite: 'Favorite',
    historyDelete: 'Delete',
    historyCopied: 'Copied',
    historyAddedToFavorites: '已添加到收藏',
    historyAlreadyInFavorites: '已在收藏中',
    historyAddFailed: '添加失败',
    historyDeleteConfirm: 'Are you sure you want to delete this translation?',
    historyDeleted: 'Deleted',
    historyDeleteFailed: 'Delete failed',
    historyFailedToLoad: 'Failed to load history',

    // Favorites page
    myFavorites: 'My Favorites',
    favoritesItems: 'items',
    favoritesNoFavoritesYet: 'No Favorites Yet',
    favoritesClickStar: 'Click the star icon next to translation results to add them to favorites',
    favoritesEditNote: '编辑备注',
    favoritesDelete: '删除',
    favoritesNote: 'Note',
    favoritesClickEditToAdd: 'Click edit to add a note...',
    favoritesAddNote: 'Add a note...',
    favoritesSave: 'Save',
    favoritesCancel: 'Cancel',
    favoritesNoteUpdated: '备注已更新',
    favoritesUpdateFailed: '更新失败',
    favoritesDeleteConfirm: 'Are you sure you want to delete this favorite?',
    favoritesFailedToLoad: 'Failed to load favorites',

    // Profile page
    accountSettings: 'Account Settings',
    profileUsername: '用户名',
    profileEmail: '邮箱',
    profileSaveChanges: 'Save Changes',
    profileCancel: 'Cancel',
    profileEditProfile: 'Edit Profile',
    changePassword: 'Change Password',
    currentPassword: 'Current Password',
    newPassword: 'New Password',
    confirmNewPassword: 'Confirm New Password',
    enterCurrentPassword: 'Enter current password',
    newPasswordPlaceholder: 'At least 8 chars with uppercase, lowercase and number',
    confirmNewPasswordPlaceholder: 'Re-enter new password',
    updatePassword: 'Update Password',
    dangerZone: 'Danger Zone',
    logout: 'Logout',
    user: 'User',
    profileTranslations: 'Translations',
    profileFavorites: 'Favorites',
    profileLanguages: 'Languages',
    profileFailedToLoad: 'Failed to load user info',
    profilePleaseFillInfo: '请填写完整信息',
    profileUpdated: 'Profile updated',
    profileUpdateFailed: 'Update failed',
    profilePleaseFillAllPasswordFields: '请填写所有密码字段',
    profilePasswordsDoNotMatch: '两次输入的新密码不一致',
    profileNewPasswordTooShort: '新密码至少需要8位',
    profilePasswordUpdated: 'Password updated, please login again',
    profilePasswordUpdateFailed: 'Password update failed',
    profileLogoutConfirm: 'Are you sure you want to logout?',
  },
  zh: {
    // Home page
    home: '首页',
    about: '关于',
    setting: '设置',
    chooseIsland: '选择您的岛屿',
    welcome: '欢迎来到 Island OS。',
    instructions: '右键拖动旋转球体。悬停在命名节点上预览每个岛屿，然后点击进入。',
    currentTheme: '当前主题: 深色模式',
    backgroundPreserved: '背景已保留',
    login: '登录',
    signup: '注册',
    dark: '深色',

    // Home sphere
    metaland: 'Metaland',
    metalandDesc: '进入鲜活的数字世界',
    transnet: 'Transnet',
    transnetDesc: '打开 AI 翻译桥梁',

    // Auth pages
    welcomeBack: '欢迎回来',
    loginOrRegister: '登录或注册以保存您的翻译历史',
    loginToSave: '登录以保存您的翻译历史',
    registerToSave: '注册以保存您的翻译历史',
    createAccount: '创建账户',
    email: '邮箱',
    password: '密码',
    username: '用户名',
    emailPlaceholder: 'your@email.com',
    passwordPlaceholder: '输入密码',
    passwordPlaceholderRegister: '至少 8 个字符，包含大小写字母和数字',
    usernamePlaceholder: '3-50 个字符，字母数字和下划线',
    processing: '处理中...',
    loginSuccessful: '登录成功',
    loginFailed: '登录失败',
    registrationSuccessful: '注册成功，请登录',
    registrationFailed: '注册失败',
    dontHaveAccount: '还没有账户？',
    alreadyHaveAccount: '已有账户？',
    passwordTooShort: '太短',
    passwordWeak: '弱',
    passwordFair: '一般',
    passwordStrong: '强',
    passwordVeryStrong: '非常强',

    // Settings page
    settings: '设置',
    settingsDescription: '使用这些设置自定义您的 Transnet 体验。',
    appearance: '外观',
    darkMode: '深色模式',
    darkModeFixed: '当前固定为深色主题',
    languageSettings: '语言',
    interfaceLanguage: '界面语言',
    languageSetViaHome: '通过首页语言选择器设置',
    configure: '配置',
    translationSettings: '翻译',
    autoDetectInput: '自动检测输入类型',
    smartDetectionEnabled: '默认启用智能检测',
    saveToHistory: '保存到历史',
    autoSaveTranslations: '自动保存翻译',
    privacy: '隐私',
    dataStorage: '数据存储',
    dataStoredSecurely: '您的翻译被安全存储',
    accountDeletion: '账户删除',
    removeDataPermanently: '永久删除您的所有数据',
    requestDeletion: '请求删除',
    settingsComingSoon: '设置即将推出：',
    settingsComingSoonDesc: '更多自定义选项将在未来的更新中提供。敬请期待增强的个性化功能。',

    // About page
    aboutTransnet: '关于 Transnet',
    aboutDescription: 'Transnet 是一个 AI 驱动的智能翻译平台，旨在跨越数字世界的语言障碍。我们的使命是让无缝沟通惠及每一个人。',
    multiLanguageSupport: '多语言支持',
    multiLanguageDesc: '使用先进的 AI 模型在英语、西班牙语、法语、德语、中文、日语和韩语之间进行翻译。',
    smartTranslation: '智能翻译',
    smartTranslationDesc: '自动检测输入类型，并为单词、短语、句子和完整文本提供上下文感知的翻译。',
    translationHistory: '翻译历史',
    translationHistoryDesc: '记录您的所有翻译，使用强大的搜索和过滤功能快速查找过去的工作。',
    favorites: '收藏',
    favoritesDesc: '保存您最好的翻译，并添加自定义注释和增强的单词含义，以便日后参考。',
    builtWithAI: '采用尖端 AI 技术构建',

    // Transnet page
    navTranslate: '翻译',
    navHistory: '历史',
    navFavorites: '收藏',
    transnetSource: '原文',
    transnetTarget: '译文',
    transnetSourcePlaceholder: '输入要翻译的文本... 按 Ctrl+Enter 快速翻译。',
    transnetTargetPlaceholder: '翻译结果将显示在这里...',
    transnetUploadImage: '点击上传图片',
    transnetOutputType: '输出类型：',
    transnetInputType: '输入类型：',
    transnetText: '文本',
    transnetImage: '图片',
    transnetTranslate: '翻译',
    transnetTranslating: '翻译中...',
    transnetContactingBackend: '正在连接 Transnet 后端...',
    transnetEnterTextBeforeTranslating: '翻译前请输入文本。',
    transnetTranslationFailed: '翻译失败。',
    transnetUnableToReach: '无法连接到 Transnet 后端。',
    transnetTranslatedAs: '已通过 {model} 翻译为 {input_type}。',
    transnetExtraOutput: '额外输出',
    transnetWordInformation: '单词信息',
    transnetWord: '单词',
    transnetPartOfSpeech: '词性',
    transnetPhonetic: '音标',
    transnetSynonyms: '同义词',
    transnetAntonyms: '反义词',
    transnetExamples: '例句',
    transnetExplanation: '解释',
    transnetMeaning: '含义',
    transnetStory: '故事',
    transnetWhenToUse: '何时使用',
    transnetHowToUse: '如何使用',
    transnetContext: '语境',
    transnetRoot: '词根',
    transnetStructure: '结构',
    transnetIdiomatic: '习语',
    transnetYes: '是',
    transnetRelationships: '关系',
    transnetRelatedWords: '相关词',
    transnetByPartOfSpeech: '按词性分类',
    transnetNouns: '名词',
    transnetVerbs: '动词',
    transnetAdjectives: '形容词',
    transnetPhraseInformation: '短语信息',
    transnetPhrase: '短语',
    transnetRelatedPhrases: '相关短语',
    transnetRelatedConcepts: '相关概念',
    transnetSentenceInformation: '句子信息',
    transnetTone: '语气',
    transnetUsage: '用法',
    transnetTranslationComparison: '翻译对比',
    transnetOriginal: '原文',
    transnetTranslation: '译文',

    // History page
    historyPageTitle: '翻译历史',
    historySource: '来源',
    historyTarget: '目标',
    historyType: '类型',
    historyNoHistoryYet: '暂无历史记录',
    historyStartTranslating: '开始翻译，您的历史记录将显示在这里',
    historyCopy: '复制',
    historyFavorite: '收藏',
    historyDelete: '删除',
    historyCopied: '已复制',
    historyAddedToFavorites: '已添加到收藏',
    historyAlreadyInFavorites: '已在收藏中',
    historyAddFailed: '添加失败',
    historyDeleteConfirm: '确定要删除此翻译吗？',
    historyDeleted: '已删除',
    historyDeleteFailed: '删除失败',
    historyFailedToLoad: '加载历史失败',

    // Favorites page
    myFavorites: '我的收藏',
    favoritesItems: '项',
    favoritesNoFavoritesYet: '暂无收藏',
    favoritesClickStar: '点击翻译结果旁的星标图标将其添加到收藏',
    favoritesEditNote: '编辑备注',
    favoritesDelete: '删除',
    favoritesNote: '备注',
    favoritesClickEditToAdd: '点击编辑添加备注...',
    favoritesAddNote: '添加备注...',
    favoritesSave: '保存',
    favoritesCancel: '取消',
    favoritesNoteUpdated: '备注已更新',
    favoritesUpdateFailed: '更新失败',
    favoritesDeleteConfirm: '确定要删除此收藏吗？',
    favoritesFailedToLoad: '加载收藏失败',

    // Profile page
    accountSettings: '账户设置',
    profileUsername: '用户名',
    profileEmail: '邮箱',
    profileSaveChanges: '保存更改',
    profileCancel: '取消',
    profileEditProfile: '编辑资料',
    changePassword: '修改密码',
    currentPassword: '当前密码',
    newPassword: '新密码',
    confirmNewPassword: '确认新密码',
    enterCurrentPassword: '输入当前密码',
    newPasswordPlaceholder: '至少8位，包含大小写字母和数字',
    confirmNewPasswordPlaceholder: '再次输入新密码',
    updatePassword: '更新密码',
    dangerZone: '危险区域',
    logout: '退出登录',
    user: '用户',
    profileTranslations: '翻译',
    profileFavorites: '收藏',
    profileLanguages: '语言',
    profileFailedToLoad: '加载用户信息失败',
    profilePleaseFillInfo: '请填写完整信息',
    profileUpdated: '资料已更新',
    profileUpdateFailed: '更新失败',
    profilePleaseFillAllPasswordFields: '请填写所有密码字段',
    profilePasswordsDoNotMatch: '两次输入的新密码不一致',
    profileNewPasswordTooShort: '新密码至少需要8位',
    profilePasswordUpdated: '密码已更新，请重新登录',
    profilePasswordUpdateFailed: '密码更新失败',
    profileLogoutConfirm: '确定要退出登录吗？',
  },
};

class LanguageService {
  private currentLanguage: Language;
  private changeListeners: Set<(lang: Language) => void> = new Set();
  private readonly STORAGE_KEY = 'transnet-language';

  constructor() {
    // Load saved language from localStorage or default to 'en'
    const saved = localStorage.getItem(this.STORAGE_KEY);
    this.currentLanguage = (saved === 'en' || saved === 'zh') ? saved : 'en';
    this.updateHtmlLangAttribute();
  }

  /**
   * Get the current language code.
   */
  getCurrentLanguage(): Language {
    return this.currentLanguage;
  }

  /**
   * Set a new language and notify all listeners.
   */
  setLanguage(language: Language): void {
    if (this.currentLanguage === language) {
      return;
    }

    this.currentLanguage = language;
    localStorage.setItem(this.STORAGE_KEY, language);
    this.updateHtmlLangAttribute();
    this.notifyListeners();
  }

  /**
   * Get translation for a key in the current language.
   */
  t(key: TranslationKey): string {
    return translations[this.currentLanguage][key] || translations.en[key] || key;
  }

  /**
   * Subscribe to language changes. Returns an unsubscribe function.
   */
  onLanguageChange(callback: (lang: Language) => void): () => void {
    this.changeListeners.add(callback);

    // Return unsubscribe function
    return () => {
      this.changeListeners.delete(callback);
    };
  }

  /**
   * Notify all registered listeners of language change.
   */
  private notifyListeners(): void {
    this.changeListeners.forEach((callback) => {
      callback(this.currentLanguage);
    });
  }

  /**
   * Update the HTML lang attribute for accessibility.
   */
  private updateHtmlLangAttribute(): void {
    document.documentElement.lang = this.currentLanguage;
  }
}

// Singleton instance
const languageService = new LanguageService();

// Export convenience functions
export const t = (key: TranslationKey): string => languageService.t(key);
export const getCurrentLanguage = (): Language => languageService.getCurrentLanguage();
export const setLanguage = (lang: Language): void => languageService.setLanguage(lang);
export const onLanguageChange = (callback: (lang: Language) => void): (() => void) =>
  languageService.onLanguageChange(callback);

// Export service instance for direct access if needed
export { languageService };
export default languageService;
