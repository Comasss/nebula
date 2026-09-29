<script>
  import { t } from '../lib/i18n.svelte.js';
  import Modal from './Modal.svelte';
  import Switch from './Switch.svelte';
  import Icon from './Icon.svelte';
  import { ui, closeDialog } from '../lib/state.svelte.js';

  const dlg = $derived(ui.dialog);
  let showPw = $state(false);

  function ok() {
    closeDialog({ ok: true, checked: dlg.checked, value: dlg.value });
  }
  function cancel() {
    closeDialog({ ok: false, checked: false, value: '' });
  }
</script>

{#if dlg}
  <Modal title={dlg.title} icon={dlg.icon} iconKind={dlg.danger ? 'err' : 'accent'} width={460} onclose={cancel}>
    {#if dlg.message}<p class="msg">{dlg.message}</p>{/if}
    {#if dlg.input}
      <div class="field">
        {#if dlg.input.label}<label for="dlg-input">{dlg.input.label}</label>{/if}
        <div class="pw">
          <!-- svelte-ignore a11y_autofocus -->
          <input
            id="dlg-input"
            class="input"
            type={dlg.input.password && !showPw ? 'password' : 'text'}
            placeholder={dlg.input.placeholder ?? ''}
            bind:value={ui.dialog.value}
            autofocus
            onkeydown={(e) => e.key === 'Enter' && ok()} />
          {#if dlg.input.password}
            <button class="icon-btn eye" onclick={() => (showPw = !showPw)} aria-label={t('common.show')}><Icon name={showPw ? 'eye-off' : 'eye'} size={16} /></button>
          {/if}
        </div>
      </div>
    {/if}
    {#if dlg.checkbox}
      <div class="check" onclick={() => (ui.dialog.checked = !dlg.checked)} role="presentation">
        <Switch checked={dlg.checked} onchange={(v) => (ui.dialog.checked = v)} label={dlg.checkbox} />
        <span>{dlg.checkbox}</span>
      </div>
    {/if}
    {#snippet footer()}
      <button class="btn ghost" onclick={cancel}>{dlg.cancelLabel}</button>
      <button class="btn {dlg.danger ? 'danger' : 'primary'}" onclick={ok}>{dlg.confirmLabel}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .msg { margin: 0 0 4px; color: var(--text-2); line-height: 1.5; white-space: pre-line; }
  .field { margin-top: 12px; }
  .pw { position: relative; }
  .pw .input { padding-right: 40px; }
  .eye { position: absolute; right: 3px; top: 3px; }
  .check { display: flex; align-items: center; gap: 12px; margin-top: 16px; cursor: pointer; color: var(--text-2); }
</style>
