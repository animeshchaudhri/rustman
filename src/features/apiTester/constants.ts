export const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] as const;

export const METHOD_COLORS: Record<string, string> = {
  GET:     "text-emerald-500 dark:text-emerald-400 border-emerald-500/30 bg-emerald-500/10",
  POST:    "text-brand-500 dark:text-brand-400 border-brand-500/30 bg-brand-500/10",
  PUT:     "text-blue-500 dark:text-blue-400 border-blue-500/30 bg-blue-500/10",
  PATCH:   "text-teal-500 dark:text-teal-400 border-teal-500/30 bg-teal-500/10",
  DELETE:  "text-red-500 dark:text-red-400 border-red-500/30 bg-red-500/10",
  HEAD:    "text-purple-500 dark:text-purple-400 border-purple-500/30 bg-purple-500/10",
  OPTIONS: "text-sky-500 dark:text-sky-400 border-sky-500/30 bg-sky-500/10",
};

export const METHOD_BADGE_DOT: Record<string, string> = {
  GET:     "text-emerald-400",
  POST:    "text-brand-400",
  PUT:     "text-blue-400",
  PATCH:   "text-teal-400",
  DELETE:  "text-red-400",
  HEAD:    "text-purple-400",
  OPTIONS: "text-sky-400",
};

export const METHOD_BADGE_INLINE: Record<string, string> = {
  GET:     "text-emerald-500 dark:text-emerald-400 bg-emerald-500/10",
  POST:    "text-brand-500 dark:text-brand-400 bg-brand-400/10",
  PUT:     "text-blue-500 dark:text-blue-400 bg-blue-400/10",
  PATCH:   "text-teal-500 dark:text-teal-400 bg-teal-400/10",
  DELETE:  "text-red-500 dark:text-red-400 bg-red-400/10",
  HEAD:    "text-purple-500 dark:text-purple-400 bg-purple-400/10",
  OPTIONS: "text-sky-500 dark:text-sky-400 bg-sky-400/10",
};
