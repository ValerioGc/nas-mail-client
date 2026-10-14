import { afterEach, expect, it, vi } from 'vitest';
import {
  deleteMail,
  emptyTrash,
  fetchMails,
  fetchSettings,
  fetchStatus,
  moveMail,
  saveSettings,
  sendTestMail,
} from '@/services/mailApi';
import { answer, mail, mailSettings, settingsView } from '@tests/fixtures/mail';

afterEach(() => vi.unstubAllGlobals());

function stub(...responses: Promise<Response>[]) {
  const fetcher = vi.fn();
  for (const response of responses) {
    fetcher.mockReturnValueOnce(response);
  }
  vi.stubGlobal('fetch', fetcher);
  return fetcher;
}

it('reads the status, the settings and a folder from relative endpoints', async () => {
  const fetcher = stub(
    answer({ msmtp: true, configured: false, sender: false }),
    answer(settingsView()),
    answer([mail()]),
  );
  await expect(fetchStatus()).resolves.toEqual({ msmtp: true, configured: false, sender: false });
  await expect(fetchSettings()).resolves.toEqual(settingsView());
  await expect(fetchMails('trash')).resolves.toEqual([mail()]);
  expect(fetcher.mock.calls.map(([path]) => path)).toEqual([
    'api/status',
    'api/settings',
    'api/mails?folder=trash',
  ]);
});

it('saves the settings as JSON and sends the test mail', async () => {
  const settings = mailSettings();
  const fetcher = stub(answer(settingsView()), answer(mail({ category: 'test' })));
  await saveSettings(settings);
  expect(fetcher).toHaveBeenNthCalledWith(1, 'api/settings', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(settings),
  });
  await expect(sendTestMail()).resolves.toMatchObject({ category: 'test' });
  expect(fetcher).toHaveBeenLastCalledWith('api/test', { method: 'POST' });
});

it('moves, deletes and empties with the matching verbs', async () => {
  const fetcher = stub(
    answer(undefined, 204),
    answer(undefined, 204),
    answer(undefined, 204),
    answer({ removed: 2 }),
  );
  await moveMail(3, 'trash');
  await moveMail(3, 'sent');
  await deleteMail(3);
  await emptyTrash();
  expect(fetcher.mock.calls).toEqual([
    ['api/mails/3/trash', { method: 'POST' }],
    ['api/mails/3/restore', { method: 'POST' }],
    ['api/mails/3', { method: 'DELETE' }],
    ['api/trash', { method: 'DELETE' }],
  ]);
});

it('throws the error of the backend, or says the answer is not what it expected', async () => {
  stub(
    answer({ error: 'The sender address is not valid.' }, 400),
    answer(undefined, 500),
    Promise.resolve(new Response('<html>', { status: 200 })),
    answer([{ id: 'x' }]),
  );
  await expect(fetchSettings()).rejects.toThrow('The sender address is not valid.');
  await expect(fetchStatus()).rejects.toThrow('HTTP 500');
  await expect(fetchStatus()).rejects.toThrow('Invalid backend answer (HTTP 200).');
  await expect(fetchMails('sent')).rejects.toThrow('Invalid backend answer.');
});
