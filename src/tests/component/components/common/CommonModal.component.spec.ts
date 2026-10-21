import { flushPromises, mount } from '@vue/test-utils';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { h } from 'vue';
import CommonModal from '@/components/common/CommonModal.vue';
import { setLocale } from '@/i18n';

beforeEach(() => setLocale('it'));
afterEach(() => {
  document.body.innerHTML = '';
});

it('opens with its title, as a native modal dialog', async () => {
  const wrapper = mount(CommonModal, {
    props: { modelValue: true, title: 'Impostazioni' },
    slots: { default: () => h('p', 'contenuto') },
    attachTo: document.body,
  });
  await flushPromises();
  const dialog = wrapper.get('dialog');
  expect(dialog.attributes('aria-label')).toBe('Impostazioni');
  expect(wrapper.get('h2').text()).toBe('Impostazioni');
  expect(wrapper.text()).toContain('contenuto');
  expect(dialog.attributes('open')).toBeDefined();
});

it('closes with the button, with Escape and from the backdrop, not from inside', async () => {
  const wrapper = mount(CommonModal, {
    props: { modelValue: true, title: 'Impostazioni', 'onUpdate:modelValue': () => undefined },
  });
  await wrapper.get('[data-testid="modal_close"]').trigger('click');
  await wrapper.get('dialog').trigger('cancel');
  await wrapper.get('.modal').trigger('click');
  await wrapper.get('dialog').trigger('click');

  expect(wrapper.emitted('update:modelValue')).toEqual([[false], [false], [false]]);
  await wrapper.setProps({ modelValue: false });
  expect(wrapper.find('dialog').exists()).toBe(false);
});
