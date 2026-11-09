import type { CSSProperties } from "react";
import type { ViewKey } from "./types";

export type ThemeGroup = "Couleurs" | "Dégradés" | "Images";

export interface AppTheme {
  id: string;
  label: string;
  group: ThemeGroup;
  background: string;
  dark?: boolean;
  image?: boolean;
  overlay?: string;
}

export type ThemePreferences = Record<ViewKey, string>;

const imageTheme = (id: string, label: string, file: string, dark = false): AppTheme => ({
  id,
  label,
  group: "Images",
  background: `url('/themes/${file}')`,
  image: true,
  dark,
  overlay: dark
    ? "linear-gradient(135deg, rgb(5 18 39 / 18%), rgb(13 31 61 / 30%))"
    : "linear-gradient(135deg, rgb(255 255 255 / 12%), rgb(246 250 255 / 24%))",
});

export const themes: AppTheme[] = [
  { id: "color-paper", label: "Papier", group: "Couleurs", background: "#f5f6f8" },
  { id: "color-blue", label: "Bleu", group: "Couleurs", background: "#dbeafe" },
  { id: "color-sage", label: "Sauge", group: "Couleurs", background: "#dfece4" },
  { id: "color-sand", label: "Sable", group: "Couleurs", background: "#f4e8d5" },
  { id: "color-lilac", label: "Lilas", group: "Couleurs", background: "#e9e1f6" },
  { id: "color-rose", label: "Rose", group: "Couleurs", background: "#f5dfe5" },
  { id: "color-mint", label: "Menthe", group: "Couleurs", background: "#d9f0ea" },
  { id: "color-slate", label: "Ardoise", group: "Couleurs", background: "#34445f", dark: true },

  { id: "gradient-sky", label: "Ciel", group: "Dégradés", background: "linear-gradient(135deg, #dbeafe 0%, #f0e7ff 52%, #ffe4e6 100%)" },
  { id: "gradient-lagoon", label: "Lagon", group: "Dégradés", background: "linear-gradient(145deg, #b9f3e4 0%, #c7e8ff 48%, #b9c8ff 100%)" },
  { id: "gradient-sunset", label: "Coucher", group: "Dégradés", background: "linear-gradient(140deg, #ffd7b8 0%, #f5b8c8 48%, #c8b8f5 100%)" },
  { id: "gradient-forest", label: "Forêt", group: "Dégradés", background: "linear-gradient(145deg, #dcefe2 0%, #acd6c2 50%, #79a9a1 100%)" },
  { id: "gradient-berry", label: "Baies", group: "Dégradés", background: "linear-gradient(145deg, #5f3a86 0%, #a34c82 52%, #e18b93 100%)", dark: true },
  { id: "gradient-night", label: "Minuit", group: "Dégradés", background: "linear-gradient(145deg, #172554 0%, #243b78 48%, #513d83 100%)", dark: true },
  { id: "gradient-amber", label: "Ambre", group: "Dégradés", background: "linear-gradient(145deg, #fff2c7 0%, #ffc98b 50%, #ef9f83 100%)" },
  { id: "gradient-arctic", label: "Arctique", group: "Dégradés", background: "linear-gradient(145deg, #e9f7ff 0%, #c8edf0 48%, #cdd8f8 100%)" },

  imageTheme("image-dawn", "Aube", "01-aube.jpg"),
  imageTheme("image-ocean", "Océan", "02-ocean.jpg", true),
  imageTheme("image-forest", "Forêt brumeuse", "03-foret.jpg"),
  imageTheme("image-night", "Nuit", "04-nuit.jpg", true),
  imageTheme("image-desert", "Désert", "05-desert.jpg"),
  imageTheme("image-leaves", "Feuillage", "06-feuillage.jpg"),
  imageTheme("image-geometry", "Géométrie", "07-geometrie.jpg"),
  imageTheme("image-mountain", "Montagne", "08-montagne.jpg"),
  imageTheme("image-lake", "Lac", "09-lac.jpg", true),
  imageTheme("image-clouds", "Nuages", "10-nuages.jpg"),
  imageTheme("image-meadow", "Prairie", "11-prairie.jpg"),
  imageTheme("image-aurora", "Aurore boréale", "12-aurore.jpg", true),
  imageTheme("image-coral", "Corail", "13-corail.jpg"),
  imageTheme("image-ink", "Encre", "14-encre.jpg"),
  imageTheme("image-city", "Ville", "15-ville.jpg", true),
];

export const defaultThemePreferences: ThemePreferences = {
  today: "image-dawn",
  todo: "gradient-sky",
  important: "gradient-sunset",
  later: "image-desert",
  completed: "color-sage",
};

const storageKey = "notion-tasks-themes-v1";

export function loadThemePreferences(): ThemePreferences {
  try {
    const saved = JSON.parse(localStorage.getItem(storageKey) ?? "{}") as Partial<ThemePreferences>;
    return { ...defaultThemePreferences, ...saved };
  } catch {
    return defaultThemePreferences;
  }
}

export function saveThemePreferences(preferences: ThemePreferences) {
  localStorage.setItem(storageKey, JSON.stringify(preferences));
}

export function themeStyle(theme: AppTheme): CSSProperties {
  const background = theme.overlay ? `${theme.overlay}, ${theme.background}` : theme.background;
  return {
    background,
    backgroundSize: theme.image ? "cover" : undefined,
    backgroundPosition: theme.image ? "center" : undefined,
    backgroundAttachment: theme.image ? "fixed" : undefined,
  };
}
