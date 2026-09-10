import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";

export type LanguagePreference = "auto" | "zh-CN" | "en";
export type Locale = "zh-CN" | "en";

const en = {
  listening: "Listening for hooks",
  offline: "Hook service is offline",
  sources: "Claude · Codex · Gemini · Custom",
  settings: "Settings",
  collapse: "Collapse to edge capsule",
  expand: "Open notification panel",
  online: "Hook service online",
  offlineLabel: "Hook service offline",
  waiting: "Waiting for you",
  pending: "Pending",
  remove: "Remove",
  details: "Current notification details",
  openFolder: "Open project folder",
  openLink: "Open related link",
  openTerminal: "Open the originating terminal pane",
  answerPlaceholder: "Enter an answer",
  customAnswerPlaceholder: "Or enter another answer",
  secretAnswerPlaceholder: "Enter a private answer",
  submitAnswer: "Submit answer",
  cancel: "Cancel",
  allowOnce: "Allow once",
  deny: "Deny",
  submitFailed: "Could not submit",
  terminalHandoff: "This source cannot return answers through hooks yet. Answer in the original terminal.",
  handled: "Handled",
  firstNotification: "Waiting for the first notification",
  setupHint: "Connect Claude, Codex, Gemini, or your own local HTTP client in Settings.",
  openSettings: "Open settings",
  recent: "Recent notifications",
  notificationHub: "Notification hub",
  windowsToasts: "Windows notifications",
  windowsToastsHint: "Show a toast for completed, failed, and attention events",
  autoExpand: "Expand for actions",
  autoExpandHint: "Show the panel when approval or an answer is required",
  launchAtLogin: "Launch at sign-in",
  launchAtLoginHint: "Run quietly in the tray after Windows sign-in",
  language: "Language",
  languageHint: "Follow Windows or choose a display language",
  automatic: "Windows default",
  chinese: "简体中文",
  english: "English",
  hookConnections: "Hook connections",
  connected: "Notifications connected",
  disconnected: "Not connected",
  connect: "Connect",
  disconnect: "Disconnect",
  sourceProfiles: "Source profiles",
  enabled: "Enabled",
  toast: "Toast",
  sound: "Sound",
  sourceExpand: "Expand",
  name: "Name",
  icon: "Icon",
  color: "Color",
  customHook: "Custom Hook",
  customHookHint: "Copy command after installing",
  copyCommand: "Copy custom Hook command",
  sendTest: "Send test notification",
  saved: "Saved",
} as const;

type TranslationKey = keyof typeof en;

const zh: Record<TranslationKey, string> = {
  listening: "正在监听 Hook",
  offline: "Hook 服务未启动",
  sources: "Claude · Codex · Gemini · 自定义",
  settings: "设置",
  collapse: "收起为贴边胶囊",
  expand: "展开通知面板",
  online: "Hook 服务在线",
  offlineLabel: "Hook 服务离线",
  waiting: "等待你的处理",
  pending: "待处理",
  remove: "移除",
  details: "当前通知详情",
  openFolder: "打开项目目录",
  openLink: "打开关联链接",
  openTerminal: "打开原终端 Pane",
  answerPlaceholder: "输入回答",
  customAnswerPlaceholder: "或输入其他回答",
  secretAnswerPlaceholder: "输入保密回答",
  submitAnswer: "提交回答",
  cancel: "取消",
  allowOnce: "允许一次",
  deny: "拒绝",
  submitFailed: "提交失败",
  terminalHandoff: "此来源暂不支持通过 Hook 回传答案，请在原终端中回答。",
  handled: "已处理",
  firstNotification: "等待第一条通知",
  setupHint: "在设置中接入 Claude、Codex、Gemini 或本机 HTTP 客户端。",
  openSettings: "打开设置",
  recent: "最近通知",
  notificationHub: "通知中枢",
  windowsToasts: "Windows 系统通知",
  windowsToastsHint: "任务完成、失败和等待操作时显示 Toast",
  autoExpand: "等待操作时展开",
  autoExpandHint: "权限和问题到达时自动显示悬浮面板",
  launchAtLogin: "开机启动",
  launchAtLoginHint: "登录 Windows 后在托盘中静默运行",
  language: "语言",
  languageHint: "跟随 Windows 或选择显示语言",
  automatic: "跟随系统",
  chinese: "简体中文",
  english: "English",
  hookConnections: "Hook 接入",
  connected: "已接管通知",
  disconnected: "尚未接入",
  connect: "接入",
  disconnect: "卸载",
  sourceProfiles: "来源配置",
  enabled: "启用",
  toast: "通知",
  sound: "声音",
  sourceExpand: "展开",
  name: "名称",
  icon: "图标",
  color: "颜色",
  customHook: "自定义 Hook",
  customHookHint: "安装后可复制通用命令",
  copyCommand: "复制自定义 Hook 命令",
  sendTest: "发送测试通知",
  saved: "已保存",
};

export function resolveLocale(preference: LanguagePreference): Locale {
  if (preference !== "auto") return preference;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export function translator(preference: LanguagePreference) {
  const locale = resolveLocale(preference);
  const dictionary = locale === "zh-CN" ? zh : en;
  return { locale, t: (key: TranslationKey) => dictionary[key] };
}

const I18nContext = createContext(translator("auto"));

export function I18nProvider({ preference, children }: { preference: LanguagePreference; children: ReactNode }) {
  const value = useMemo(() => translator(preference), [preference]);
  useEffect(() => {
    document.documentElement.lang = value.locale;
  }, [value.locale]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  return useContext(I18nContext);
}
