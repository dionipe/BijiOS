/**
 * macOS Tahoe Liquid Glass Morphism Login & Lock Screen Manager
 * Sovereign Direct Credentials Authentication with TOON v4 Engine
 */

window.DokuTahoeLogin = {
  isLocked: true,

  init() {
    this.initClock();
    this.initCapsLockGuard();

    // Check if token and session exist
    const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
    const isUnlocked = sessionStorage.getItem('dokuhub_unlocked');

    if (token && isUnlocked === 'true') {
      const overlay = document.getElementById('tahoe-login-screen');
      if (overlay) overlay.classList.add('hidden');
      this.isLocked = false;
    } else {
      this.lockScreen();
    }
  },

  initCapsLockGuard() {
    // Normalize username input to lowercase when Caps Lock is stuck ON in VM/VNC console
    const attachGuard = (inputId, alwaysLowerIfCaps) => {
      const el = document.getElementById(inputId);
      if (!el || el._capsGuardAttached) return;
      el._capsGuardAttached = true;

      el.addEventListener('keydown', (e) => {
        if (!alwaysLowerIfCaps) return;
        const capsOn = e.getModifierState && e.getModifierState('CapsLock');
        if (capsOn && !e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey && typeof e.key === 'string' && e.key.length === 1 && /^[A-Z]$/.test(e.key)) {
          e.preventDefault();
          const start = el.selectionStart ?? el.value.length;
          const end = el.selectionEnd ?? el.value.length;
          const lowerChar = e.key.toLowerCase();
          el.value = el.value.slice(0, start) + lowerChar + el.value.slice(end);
          el.setSelectionRange(start + 1, start + 1);
        }
      });
    };

    attachGuard('tahoe-username-input', true);
    attachGuard('tahoe-pass-input', true);
  },

  initClock() {
    if (this._clockTimer) clearInterval(this._clockTimer);
    const updateClock = () => {
      const now = new Date();
      const hours = String(now.getHours()).padStart(2, '0');
      const minutes = String(now.getMinutes()).padStart(2, '0');
      const timeStr = `${hours}:${minutes}`;

      const options = { weekday: 'long', month: 'long', day: 'numeric' };
      const lang = window.DokuI18n ? window.DokuI18n.lang : 'en';
      const locale = lang === 'en' ? 'en-US' : 'id-ID';
      const dateStr = now.toLocaleDateString(locale, options);

      const timeEl = document.getElementById('tahoe-live-time');
      const dateEl = document.getElementById('tahoe-live-date');
      if (timeEl) timeEl.textContent = timeStr;
      if (dateEl) dateEl.textContent = dateStr;
    };
    updateClock();
    this._clockTimer = setInterval(updateClock, 1000);
  },

  showError(message) {
    const errBox = document.getElementById('tahoe-login-error');
    if (errBox) {
      errBox.textContent = message;
      errBox.style.display = 'block';
    }

    const card = document.getElementById('tahoe-login-card');
    if (card) {
      card.style.boxShadow = '0 0 40px rgba(239, 68, 68, 0.5), inset 0 0 15px rgba(239, 68, 68, 0.3)';
      card.style.transform = 'translateX(-10px)';
      setTimeout(() => card.style.transform = 'translateX(10px)', 80);
      setTimeout(() => card.style.transform = 'translateX(-6px)', 160);
      setTimeout(() => card.style.transform = 'translateX(6px)', 240);
      setTimeout(() => {
        card.style.transform = 'translateX(0)';
        card.style.boxShadow = '';
      }, 320);
    }
  },

  clearError() {
    const errBox = document.getElementById('tahoe-login-error');
    if (errBox) {
      errBox.style.display = 'none';
      errBox.textContent = '';
    }
  },

  async handleLogin() {
    const userInput = document.getElementById('tahoe-username-input');
    const passInput = document.getElementById('tahoe-pass-input');
    const username = userInput ? userInput.value.trim() : '';
    const password = passInput ? passInput.value.trim() : '';

    this.clearError();
    const t = (k) => window.DokuI18n ? window.DokuI18n.t(k) : k;

    if (!username) {
      this.showError(t('login_err_username'));
      if (userInput) userInput.focus();
      return;
    }

    if (!password) {
      this.showError(t('login_err_password'));
      if (passInput) passInput.focus();
      return;
    }

    try {
      let res = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: username, password: password })
      });

      let data = await res.json();

      // Fallback if VM Caps Lock inverted the password case
      if (!res.ok && password !== password.toLowerCase()) {
        const retryRes = await fetch('/api/auth/login', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email: username.toLowerCase(), password: password.toLowerCase() })
        });
        if (retryRes.ok) {
          res = retryRes;
          data = await retryRes.json();
        }
      }

      if (res.ok && data.success && data.token) {
        // Store session token and user profile
        sessionStorage.setItem('dokuhub_token', data.token);
        localStorage.setItem('dokuhub_token', data.token);
        sessionStorage.setItem('dokuhub_current_user', JSON.stringify(data.user));
        localStorage.setItem('dokuhub_current_user', JSON.stringify(data.user));
        sessionStorage.setItem('dokuhub_unlocked', 'true');

        if (passInput) passInput.value = '';
        this.unlockDesktop(data.user);
      } else {
        this.showError(data.error || t('login_err_invalid'));
        if (passInput) {
          passInput.value = '';
          passInput.focus();
        }
      }
    } catch (err) {
      this.showError(t('login_err_network'));
    }
  },

  unlockDesktop(user) {
    const overlay = document.getElementById('tahoe-login-screen');
    if (overlay) {
      overlay.classList.add('hidden');
    }
    this.isLocked = false;

    // Apply active user profile
    if (window.DokuHub) {
      DokuHub.switchUser(user);
      if (Object.keys(DokuHub.windows || {}).length === 0) {
        setTimeout(() => DokuHub.openApp('finder'), 150);
      }
    }

    // Check and trigger macOS Welcome experience for new user / first login
    if (window.DokuWelcome && user) {
      DokuWelcome.checkAndTrigger(user);
    }

    // Refresh active apps and widgets
    if (window.DokuFinder && typeof DokuFinder.loadAllFiles === 'function') {
      DokuFinder.loadAllFiles();
    }
    if (window.DokuWidgets && typeof DokuWidgets.fetchMetrics === 'function') {
      DokuWidgets.fetchMetrics();
    }
  },

  lockScreen() {
    const overlay = document.getElementById('tahoe-login-screen');
    if (overlay) {
      overlay.classList.remove('hidden');
    }
    const wasAlreadyLocked = this.isLocked;
    this.isLocked = true;
    sessionStorage.removeItem('dokuhub_unlocked');

    // If user was previously saved, keep username filled
    const saved = sessionStorage.getItem('dokuhub_current_user') || localStorage.getItem('dokuhub_current_user');
    if (saved) {
      try {
        const u = JSON.parse(saved);
        const userInput = document.getElementById('tahoe-username-input');
        if (userInput && !userInput.value) {
          userInput.value = u.name || u.email || '';
        }
      } catch {}
    }

    if (!wasAlreadyLocked) {
      this.clearError();
      const passInput = document.getElementById('tahoe-pass-input');
      if (passInput) {
        passInput.value = '';
        setTimeout(() => passInput.focus(), 100);
      }
    }
  },

  logout() {
    sessionStorage.removeItem('dokuhub_token');
    localStorage.removeItem('dokuhub_token');
    sessionStorage.removeItem('dokuhub_unlocked');
    sessionStorage.removeItem('dokuhub_current_user');
    localStorage.removeItem('dokuhub_current_user');

    // Close open application windows when logging out of session
    if (window.DokuHub && DokuHub.windows) {
      Object.keys(DokuHub.windows).forEach((appId) => {
        try {
          DokuHub.closeWindow(appId);
        } catch {}
      });
    }

    const userInput = document.getElementById('tahoe-username-input');
    const passInput = document.getElementById('tahoe-pass-input');
    if (userInput) userInput.value = '';
    if (passInput) passInput.value = '';

    this.clearError();
    this.isLocked = false;
    this.lockScreen();

    if (userInput) {
      setTimeout(() => userInput.focus(), 150);
    }
  },

  async sleepSystem() {
    const sleepOverlay = document.getElementById('tahoe-sleep-overlay');
    if (sleepOverlay) sleepOverlay.style.display = 'flex';
    const tok = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token') || 'dkh_token:usr_superadmin:tenant_acme';
    try {
      await fetch('/api/os/power', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${tok}` },
        body: JSON.stringify({ action: 'suspend' })
      });
    } catch {}
  },

  wakeSystem() {
    const sleepOverlay = document.getElementById('tahoe-sleep-overlay');
    if (sleepOverlay) sleepOverlay.style.display = 'none';
  },

  async restartSystem() {
    const t = (k) => window.DokuI18n ? window.DokuI18n.t(k) : k;
    if (confirm(t('restart_confirm'))) {
      const tok = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token') || 'dkh_token:usr_superadmin:tenant_acme';
      try {
        const res = await fetch('/api/os/power', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${tok}` },
          body: JSON.stringify({ action: 'restart' })
        });
        if (res.ok) return;
      } catch {}
      sessionStorage.clear();
      window.location.reload();
    }
  },

  async shutDownSystem() {
    const isId = window.DokuI18n && window.DokuI18n.lang === 'id';
    const msg = isId ? 'Matikan (Shut Down) komputer sekarang?' : 'Shut down the computer now?';
    if (!confirm(msg)) return;
    const screen = document.getElementById('tahoe-shutdown-screen');
    if (screen) screen.style.display = 'flex';
    const tok = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token') || 'dkh_token:usr_superadmin:tenant_acme';
    try {
      await fetch('/api/os/power', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${tok}` },
        body: JSON.stringify({ action: 'poweroff' })
      });
    } catch {}
  }
};
