// Multiplayer (the old launcher's page, which the old Omsi-Hub opened from the start's links):
// hosting the next duty, joining a friend's session by its code, and servers that are always on
// (`hub_lan_*`: openOMSI's own network layer; the game opens and joins the sessions itself,
// `--lan-host` and `--lan-join`).

import { html, useState, useEffect } from '../../vendor/preact-htm.js';
import { S, useStore, set, toast } from '../store.js';
import { t } from '../i18n.js';
import { call } from '../api.js';
import { Card, Input, Spinner } from '../ui.js';
import { Button, Icon, Seg, Switch } from '../hub/bits.js';
import { choose, serverJoined, allowedBus } from '../hub/drive.js';

const Field = ({ label, children }) => html`<label class="field"><span class="label">${t(label)}</span>${children}</label>`;

export function MultiplayerPage() {
  const [tab, setTab] = useState('code');
  return html`
    <div class="stack">
      <${Seg} value=${tab} options=${[['code', 'Connect by Code'], ['servers', 'Servers']]} onChange=${setTab} />
      ${tab === 'code' ? html`<${ByCode} />` : html`<${Servers} />`}
    </div>
  `;
}

/** The session a game on this computer hosts now (its code), asked every two seconds. */
function useHosted() {
  const [hosted, setHosted] = useState(null);
  useEffect(() => {
    let on = true;
    const ask = () => call('hub_lan_hosted').then((l) => on && setHosted((l || [])[0] || null)).catch(() => {});
    ask();
    const id = setInterval(ask, 2000);
    return () => ((on = false), clearInterval(id));
  }, []);
  return hosted;
}

function ByCode() {
  const s = useStore();
  const c = s.choice;
  const hosting = c.lan_mode === 'host';
  const session = useHosted();
  const [code, setCode] = useState(c.lan_mode === 'join' ? c.lan_addr : '');
  const [check, setCheck] = useState(null);
  const [host, setHost] = useState(null);
  useEffect(() => {
    setHost(null);
    if (!code.trim()) return setCheck(null);
    const asked = code.trim();
    // (an answer for a code typed before is not taken: the one typed since may answer first)
    let live = true;
    const id = setTimeout(() => {
      call('hub_lan_check', { text: code }).then((c) => live && setCheck(c)).catch(() => live && setCheck(null));
      if (/^OMSI-/i.test(asked)) call('hub_lan_query', { address: asked }).then((h) => live && setHost({ ...h, code: asked })).catch(() => live && setHost(null));
    }, 300);
    return () => ((live = false), clearTimeout(id));
  }, [code]);
  const join = () => {
    if (!code.trim()) return toast(t('Paste a session code first'), 'error');
    const known = host?.code === code.trim() ? host : null;
    if (known?.map && S.maps?.some((m) => m.file.toLowerCase() === known.map.replace(/\\/g, '/').toLowerCase())) choose({ map: known.map.replace(/\\/g, '/') });
    choose({ lan_mode: 'join', lan_addr: code.trim(), lan_map: known?.map || '', ...serverJoined(known) });
    choose(allowedBus());
    set({ page: 'drive' });
  };
  return html`<div class="grid-2" style="align-items:start">
    <${Card} title=${t('Host a game')}>
      <div class="stack">
        <p class="muted">${t('Your next duty opens a session. Give the code to your friends: it works at home and over the internet.')}</p>
        <${Switch} checked=${hosting} label="Host my next duty" onChange=${(on) => {
          choose({ lan_mode: on ? 'host' : 'off' });
          if (on) call('hub_lan_tunnel').catch(() => {});
        }} />
        ${session
          ? html`<div class="field"><span class="label">${t('Session code')}</span>
              <div class="row"><code class="input selectable" style="display:flex;align-items:center">${session.code}</code>
              <${Button} icon="content_copy" title="Copy the code" onClick=${() => navigator.clipboard?.writeText(session.code).then(() => toast(t('Session code copied'), 'ok')).catch(() => {})} /></div>
              <span class="help">${t(session.tunnel ? 'Friends can join from anywhere' : 'Getting ready for friends on the internet...')}</span></div>`
          : html`<p class="dim small">${t(hosting ? 'Start a duty on the Drive page: the code appears here.' : 'Turn on hosting, then start a duty on the Drive page.')}</p>`}
        <div><${Button} icon="directions_bus" label="Go to Drive" onClick=${() => set({ page: 'drive' })} /></div>
      </div>
    </${Card}>
    <${Card} title=${t('Connect by Code')}>
      <div class="stack">
        <p class="muted">${t("Paste the code your friend's game shows. The map, time and weather are the host's; you choose your bus.")}</p>
        <${Input} value=${code} placeholder="OMSI-XXXX-XXXX-..." raw onInput=${setCode} />
        ${check && html`<div class=${'small ' + (check.ok ? 'dim' : '')} style=${check.ok ? '' : 'color:var(--hub-danger)'}>${check.text}</div>`}
        ${host && html`<div class="notice"><${Icon} name="groups" /><div class="grow"><b>${host.name}</b><div class="muted small">${host.map} - ${t('%{n} of %{max} players', { n: host.players, max: host.max_players })}</div></div></div>`}
        <div><${Button} kind="primary" icon="play_arrow" label="Choose a bus and join" onClick=${join} /></div>
      </div>
    </${Card}>
  </div>`;
}

function Servers() {
  const s = useStore();
  const [list, setList] = useState(null);
  const [info, setInfo] = useState({});
  const [address, setAddress] = useState('');
  const [name, setName] = useState('');
  const ask = (a) => {
    setInfo((x) => ({ ...x, [a]: { loading: true } }));
    call('hub_lan_query', { address: a })
      .then((r) => setInfo((x) => ({ ...x, [a]: { ok: r } })))
      .catch((e) => setInfo((x) => ({ ...x, [a]: { err: e.message } })));
  };
  useEffect(() => {
    call('hub_lan_servers').then((l) => {
      if (!Array.isArray(l)) return;
      setList(l);
      l.forEach((x) => ask(x.address));
    }).catch(() => setList([]));
  }, []);
  const save = (l) => {
    setList(l);
    call('hub_lan_servers_save', { list: l }).catch((e) => toast(e.message, 'error'));
  };
  const add = () => {
    const a = address.trim();
    if (!a) return toast(t("Type the server's address (an IP, a name or a link)"), 'error');
    // (not before the list came: saved without it, the servers kept would be lost)
    if (!list) return;
    if (list.some((x) => x.address === a)) return toast(t('That server is in the list already'), 'error');
    save([...list, { name: name.trim() || a, address: a }]);
    setAddress('');
    setName('');
    ask(a);
  };
  const join = (x) => {
    const i = info[x.address]?.ok;
    if (!i) return toast(t('The server has not answered yet (is its address right? is it running?)'), 'error');
    const map = (i.map || '').replace(/\\/g, '/');
    if (map && s.maps?.some((m) => m.file.toLowerCase() === map.toLowerCase())) choose({ map });
    else if (map) toast(t('The server plays %{map}, which is not installed here: it is fetched from the server on joining.', { map }));
    const web = /^(https?|wss?):\/\//.test(x.address) || x.address === 'openomsi';
    choose({ lan_mode: 'join', lan_addr: !web && i.reached_at ? i.reached_at : x.address, lan_map: i.map || '', ...serverJoined({ ...i, name: x.name || i.name }) });
    choose(allowedBus());
    toast(t('Joined %{name} - choose your bus and duty, then Start the duty', { name: x.name || i.name }), 'ok');
    set({ page: 'drive' });
  };
  return html`<div class="stack">
    <${Card} title=${t('Add server')}>
      <div class="row">
        <div class="grow"><${Field} label="Server address"><${Input} value=${address} placeholder="play.example.org" raw onInput=${setAddress} onKeyDown=${(e) => e.key === 'Enter' && add()} /></${Field}></div>
        <div style="width:240px"><${Field} label="Name (optional)"><${Input} value=${name} onInput=${setName} /></${Field}></div>
        <div style="align-self:flex-end"><${Button} icon="add" label="Add server" onClick=${add} /></div>
      </div>
    </${Card}>
    ${list == null
      ? html`<div class="row muted"><${Spinner} /></div>`
      : list.length === 0
        ? html`<${Card}><div class="empty"><${Icon} name="public" /><div>${t('No servers yet. Add a server by the address its owner gives you.')}</div></div></${Card}>`
        : list.map((x) => {
            const st = info[x.address] || {};
            const i = st.ok;
            return html`<${Card}><div class="row" style="align-items:flex-start;gap:14px">
              ${i?.icon ? html`<img src=${i.icon} width="48" height="48" style="border-radius:8px" />` : html`<div class="avatar" style="width:48px;height:48px;border-radius:8px"><${Icon} name="public" /></div>`}
              <div class="grow stack" style="gap:4px;min-width:0">
                <div class="row"><b style="font-size:15px">${i?.name || x.name}</b>${i?.password && html`<span class="tag">${t('Password')}</span>`}</div>
                ${st.loading && html`<div class="row muted small"><${Spinner} /> ${t('Asking the server...')}</div>`}
                ${st.err && html`<div class="small" style="color:var(--hub-danger)">${t("Can't reach the server: %{err}", { err: st.err })}</div>`}
                ${i && html`<div class="muted small">${[i.map, t('%{n} of %{max} players', { n: i.players, max: i.max_players }), i.time, i.weather].filter(Boolean).join(' - ')}</div>`}
                ${i?.motd && html`<div class="dim small">${i.motd}</div>`}
              </div>
              <${Button} small icon="refresh" title="Refresh" onClick=${() => ask(x.address)} />
              ${x.address !== 'openomsi' && html`<${Button} small icon="delete" title="Remove from the list" onClick=${() => save(list.filter((y) => y !== x))} />`}
              <${Button} kind="primary" icon="play_arrow" label="Join" disabled=${!i} onClick=${() => join(x)} />
            </div></${Card}>`;
          })}
  </div>`;
}
