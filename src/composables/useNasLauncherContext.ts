import { computed } from 'vue';
import { useNasLauncherContext as useLauncherContext } from '@valeriogc/nas-base-app';
import { locale, messages, setLocale } from '@/i18n';

/** Binds the launcher context to the message catalogue of the app. */
export function useNasLauncherContext() {
  const { active } = useLauncherContext({ onLocaleChange: setLocale });
  const t = computed(() => messages[locale.value]);

  return { locale, t, active };
}
