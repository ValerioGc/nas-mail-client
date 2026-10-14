import { expect, test } from '@playwright/test';

const settings = {
  language: 'it',
  smtp: {
    host: '',
    port: 465,
    security: 'tls',
    username: '',
    from: '',
    recipients: [],
  },
  access: { refused: false },
  ui: { available: false, updated: false, failed: false },
  apps: { available: false },
  system: { available: false, level: 'security' },
  passwordSet: false,
  configured: false,
};

test('is its first configuration until it can send, then the mailbox and its settings', async ({
  page,
}) => {
  let saved = settings;
  await page.route('**/api/settings', async (route) => {
    if (route.request().method() === 'PUT') {
      saved = { ...route.request().postDataJSON(), passwordSet: true, configured: true };
    }
    await route.fulfill({ json: saved });
  });
  await page.route('**/api/status', (route) =>
    route.fulfill({ json: { msmtp: true, configured: false, sender: false } }),
  );
  await page.route('**/api/mails?folder=*', (route) => route.fulfill({ json: [] }));
  await page.goto('/');

  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Configura l’app Email');
  await expect(page.getByTestId('section_sent')).toHaveCount(0);
  await page.getByPlaceholder('smtp.example.com').fill('smtp.example.com');
  await page.getByPlaceholder('nas@example.com').fill('nas@example.com');
  await page.getByLabel('Destinatari').fill('me@example.com');
  await page.getByTestId('smtp_save').click();

  await expect(page.getByText('Nessuna mail inviata')).toBeVisible();
  await page.getByTestId('settings_open').click();
  await expect(page.getByTestId('smtp')).toBeVisible();
  await page.getByTestId('settings_tab_notifications').click();
  await expect(page.getByRole('radio').first()).toBeDisabled();
  await page.getByTestId('choice_system_available_switch').click();
  await expect(page.getByRole('radio').first()).toBeEnabled();
  await page.getByTestId('choice_apps_available_switch').click();
  await page.getByTestId('events_save').click();
  await expect(page.getByText('Impostazioni salvate.')).toBeVisible();
  expect(saved.apps).toEqual({ available: true });
  await page.getByTestId('modal_close').click();
  await expect(page.getByTestId('modal')).toHaveCount(0);
});
