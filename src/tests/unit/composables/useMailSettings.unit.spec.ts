import { afterEach, expect, it, vi } from 'vitest';
import { useMailSettings } from '@/composables/useMailSettings';
import { setLocale } from '@/i18n';
import { fetchSettings, saveSettings } from '@/services/mailApi';
import { mailSettings, settingsView } from '@tests/fixtures/mail';

vi.mock('@/services/mailApi', () => ({ fetchSettings: vi.fn(), saveSettings: vi.fn() }));

afterEach(() => setLocale('it'));

it('shares the settings it loads and keeps the error of a failed load', async () => {
  const first = useMailSettings();
  vi.mocked(fetchSettings).mockRejectedValueOnce(new Error('offline'));
  await first.load();
  expect(first.error.value).toBe('offline');

  vi.mocked(fetchSettings).mockResolvedValueOnce(settingsView());
  await first.load();
  expect(useMailSettings().settings.value).toEqual(settingsView());
  expect(first.error.value).toBeNull();
  expect(first.loading.value).toBe(false);
});

it('saves in the language of the page and reports a refused save', async () => {
  setLocale('en');
  const { save, error, settings } = useMailSettings();
  const { smtp, access, ui, apps, system } = mailSettings();
  const wanted = { smtp, access, ui, apps, system };
  vi.mocked(saveSettings).mockResolvedValueOnce(settingsView({ language: 'en' }));
  await expect(save(wanted)).resolves.toBe(true);
  expect(saveSettings).toHaveBeenCalledWith({ ...wanted, language: 'en' });
  expect(settings.value?.language).toBe('en');

  vi.mocked(saveSettings).mockRejectedValueOnce('refused');
  await expect(save(wanted)).resolves.toBe(false);
  expect(error.value).toBe('refused');
});
