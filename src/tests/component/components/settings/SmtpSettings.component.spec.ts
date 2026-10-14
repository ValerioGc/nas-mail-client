import { beforeEach, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import SmtpSettings from '@/components/settings/SmtpSettings.vue';
import { useMailSettings } from '@/composables/useMailSettings';
import { messages } from '@/i18n';
import { fetchStatus, saveSettings, sendTestMail } from '@/services/mailApi';
import { mail, settingsView } from '@tests/fixtures/mail';

vi.mock('@/services/mailApi', () => ({
  fetchSettings: vi.fn(),
  fetchStatus: vi.fn(),
  saveSettings: vi.fn(),
  sendTestMail: vi.fn(),
}));

const t = messages.it.smtp;

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(fetchStatus).mockResolvedValue({ msmtp: true, configured: true, sender: true });
  useMailSettings().settings.value = settingsView();
  useMailSettings().error.value = null;
});

it('fills the form from the settings and saves what was typed, password only when given', async () => {
  vi.mocked(saveSettings).mockResolvedValue(settingsView());
  const wrapper = mount(SmtpSettings);
  await flushPromises();
  const inputs = wrapper.findAll('input');
  expect((inputs[0]!.element as HTMLInputElement).value).toBe('smtp.example.com');
  expect(inputs[3]!.attributes('placeholder')).toBe(t.passwordKept);

  await inputs[0]!.setValue(' mail.example.org ');
  await wrapper.get('select').setValue('starttls');
  await inputs[5]!.setValue('me@example.com, you@example.com,');
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(saveSettings).toHaveBeenLastCalledWith(
    expect.objectContaining({
      smtp: expect.not.objectContaining({ password: expect.anything() }),
    }),
  );
  expect(vi.mocked(saveSettings).mock.lastCall?.[0].smtp).toMatchObject({
    host: 'mail.example.org',
    security: 'starttls',
    recipients: ['me@example.com', 'you@example.com'],
  });
  expect(wrapper.get('output').text()).toBe(t.saved);

  await inputs[3]!.setValue('new-secret');
  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(vi.mocked(saveSettings).mock.lastCall?.[0].smtp.password).toBe('new-secret');
});

it('sends a test mail and says how it went', async () => {
  vi.mocked(sendTestMail)
    .mockResolvedValueOnce(mail({ category: 'test' }))
    .mockResolvedValueOnce(mail({ status: 'failed', error: 'refused' }))
    .mockRejectedValueOnce(new Error('offline'));
  const wrapper = mount(SmtpSettings);
  await flushPromises();
  const test = () => wrapper.get('[data-testid="smtp_test"]').trigger('click');

  await test();
  await flushPromises();
  expect(wrapper.get('output').text()).toBe(t.testSent);
  await test();
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toBe(t.testFailed.replace('{error}', 'refused'));
  await test();
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toBe(t.testFailed.replace('{error}', 'offline'));
});

it('warns when msmtp is missing, and adds no text of its own to the form', async () => {
  vi.mocked(fetchStatus).mockResolvedValue({ msmtp: false, configured: false, sender: false });
  useMailSettings().settings.value = settingsView({ configured: false, passwordSet: false });
  const wrapper = mount(SmtpSettings);
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toBe(t.msmtpMissing);
  expect(wrapper.findAll('section > p')).toHaveLength(1);
  // Each field has a name, for the browser to fill it in.
  const fields = wrapper.findAll('input, select');
  expect(fields.map((field) => field.attributes('name'))).toEqual([
    'host',
    'port',
    'security',
    'username',
    'password',
    'from',
    'recipients',
  ]);
  expect(wrapper.get('[data-testid="smtp_test"]').attributes('disabled')).toBeDefined();
  expect(wrapper.findAll('input')[3]!.attributes('placeholder')).toBe('');
});

it('keeps working when the status cannot be read', async () => {
  vi.mocked(fetchStatus).mockRejectedValue(new Error('offline'));
  const wrapper = mount(SmtpSettings);
  await flushPromises();
  expect(wrapper.find('[role="alert"]').exists()).toBe(false);
});

it('welcomes a first configuration with its own heading and button', async () => {
  useMailSettings().settings.value = settingsView({ configured: false, passwordSet: false });
  const setup = mount(SmtpSettings, { props: { setup: true } });
  await flushPromises();
  expect(setup.get('h1').text()).toBe(messages.it.setup.title);
  expect(setup.get('[data-testid="smtp_save"]').text()).toBe(messages.it.setup.save);

  const tab = mount(SmtpSettings);
  await flushPromises();
  expect(tab.find('h1').exists()).toBe(false);
  expect(tab.get('[data-testid="smtp_save"]').text()).toBe(t.save);
});
