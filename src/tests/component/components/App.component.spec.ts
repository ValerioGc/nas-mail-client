import { beforeEach, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import App from '@/App.vue';
import { useMailSettings } from '@/composables/useMailSettings';
import { messages } from '@/i18n';
import { fetchMails, fetchSettings, fetchStatus, saveSettings } from '@/services/mailApi';
import { settingsView } from '@tests/fixtures/mail';

vi.mock('@/services/mailApi', () => ({
  fetchSettings: vi.fn(),
  fetchStatus: vi.fn(),
  fetchMails: vi.fn(),
  saveSettings: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  useMailSettings().settings.value = null;
  useMailSettings().error.value = null;
  vi.mocked(fetchMails).mockResolvedValue([]);
  vi.mocked(fetchStatus).mockResolvedValue({ msmtp: true, configured: true, sender: true });
});

it('opens on the sent mail once configured and moves between the folders', async () => {
  vi.mocked(fetchSettings).mockResolvedValue(settingsView());
  const wrapper = mount(App, { attachTo: document.body });
  expect(wrapper.get('output').text()).toBe(messages.it.loading);
  await flushPromises();
  expect(wrapper.get('h1').text()).toBe(messages.it.sections.sent);

  await wrapper.get('[data-testid="section_trash"]').trigger('click');
  await flushPromises();
  expect(wrapper.get('h1').text()).toBe(messages.it.sections.trash);
  expect(wrapper.find('[data-testid="modal"]').exists()).toBe(false);

  await wrapper.get('[data-testid="settings_open"]').trigger('click');
  expect(wrapper.get('[data-testid="modal"]').attributes('aria-label')).toBe(
    messages.it.settings.title,
  );
  expect(wrapper.find('[data-testid="smtp"]').exists()).toBe(true);
  wrapper.unmount();
});

it('is only its first configuration until an account can send, then the mailbox', async () => {
  vi.mocked(fetchSettings).mockResolvedValue(settingsView({ configured: false }));
  vi.mocked(saveSettings).mockResolvedValue(settingsView());
  const wrapper = mount(App);
  await flushPromises();
  expect(wrapper.find('[data-testid="setup"]').exists()).toBe(true);
  expect(wrapper.get('h1').text()).toBe(messages.it.setup.title);
  expect(wrapper.find('aside').exists()).toBe(false);

  await wrapper.get('form').trigger('submit');
  await flushPromises();
  expect(wrapper.find('[data-testid="setup"]').exists()).toBe(false);
  expect(wrapper.get('h1').text()).toBe(messages.it.sections.sent);
});

it('says when the settings cannot be read and loads them again', async () => {
  vi.mocked(fetchSettings)
    .mockRejectedValueOnce(new Error('offline'))
    .mockResolvedValueOnce(settingsView());
  const wrapper = mount(App);
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toContain('offline');
  await wrapper.get('[role="alert"] button').trigger('click');
  await flushPromises();
  expect(wrapper.get('h1').text()).toBe(messages.it.sections.sent);
});

it('opens the settings another application asked for, once there is an account', async () => {
  for (const [section, tab] of [
    ['smtp', 'account'],
    ['system', 'notifications'],
  ]) {
    window.history.replaceState({}, '', `/?section=${section}`);
    useMailSettings().settings.value = null;
    vi.mocked(fetchSettings).mockResolvedValue(settingsView());
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.get(`[data-testid="settings_tab_${tab}"]`).attributes('aria-selected')).toBe(
      'true',
    );
    wrapper.unmount();
  }

  useMailSettings().settings.value = null;
  vi.mocked(fetchSettings).mockResolvedValue(settingsView({ configured: false }));
  const first = mount(App);
  await flushPromises();
  expect(first.find('[data-testid="modal"]').exists()).toBe(false);
  expect(first.find('[data-testid="setup"]').exists()).toBe(true);
  window.history.replaceState({}, '', '/');
});
