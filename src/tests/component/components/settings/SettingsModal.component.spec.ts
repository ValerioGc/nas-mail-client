import { beforeEach, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import SettingsModal from '@/components/settings/SettingsModal.vue';
import { useMailSettings } from '@/composables/useMailSettings';
import { messages } from '@/i18n';
import { fetchStatus } from '@/services/mailApi';
import { settingsView } from '@tests/fixtures/mail';

vi.mock('@/services/mailApi', () => ({
  fetchSettings: vi.fn(),
  fetchStatus: vi.fn(),
  saveSettings: vi.fn(),
  checkSmtpConnection: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(fetchStatus).mockResolvedValue({ msmtp: true, configured: true, sender: true });
  useMailSettings().settings.value = settingsView();
});

function tab(wrapper: ReturnType<typeof mount>, name: string) {
  return wrapper.get(`[data-testid="settings_tab_${name}"]`);
}

it('has the account and the notifications in two tabs', async () => {
  const wrapper = mount(SettingsModal, {
    props: {
      modelValue: true,
      'onUpdate:modelValue': (value: boolean) => wrapper.setProps({ modelValue: value }),
    },
  });
  await flushPromises();
  expect(wrapper.findAll('[role="tab"]').map((item) => item.text())).toEqual([
    messages.it.settings.tabs.account,
    messages.it.settings.tabs.notifications,
  ]);
  expect(tab(wrapper, 'account').attributes('aria-selected')).toBe('true');
  expect(wrapper.find('[data-testid="smtp"]').exists()).toBe(true);

  await tab(wrapper, 'notifications').trigger('click');
  expect(wrapper.find('[data-testid="notifications"]').exists()).toBe(true);
  expect(wrapper.find('[data-testid="smtp"]').exists()).toBe(false);

  // Closed and opened again, it starts from the tab it was given.
  await wrapper.setProps({ modelValue: false });
  await wrapper.setProps({ modelValue: true });
  expect(tab(wrapper, 'account').attributes('aria-selected')).toBe('true');
});

it('moves between the tabs with the keyboard', async () => {
  const wrapper = mount(SettingsModal, {
    props: { modelValue: true, initialTab: 'notifications' },
    attachTo: document.body,
  });
  await flushPromises();
  const list = wrapper.get('[role="tablist"]');
  expect(tab(wrapper, 'notifications').attributes('aria-selected')).toBe('true');

  await list.trigger('keydown', { key: 'ArrowRight' });
  expect(tab(wrapper, 'account').attributes('aria-selected')).toBe('true');
  expect(document.activeElement).toBe(tab(wrapper, 'account').element);
  await list.trigger('keydown', { key: 'ArrowLeft' });
  expect(tab(wrapper, 'notifications').attributes('aria-selected')).toBe('true');
  await list.trigger('keydown', { key: 'Home' });
  expect(tab(wrapper, 'account').attributes('aria-selected')).toBe('true');
  await list.trigger('keydown', { key: 'End' });
  expect(tab(wrapper, 'notifications').attributes('aria-selected')).toBe('true');
  await list.trigger('keydown', { key: 'a' });
  expect(tab(wrapper, 'notifications').attributes('aria-selected')).toBe('true');
  wrapper.unmount();
});
