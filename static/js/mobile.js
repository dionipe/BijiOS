/**
 * DokuHub iOS-Style Mobile Responsive Controller & PWA Manager
 * Activates automatically on viewports <= 768px without altering desktop behavior.
 */

window.DokuMobile = {
  deferredPrompt: null,
  isStandalone: false,

  appCatalog: [
    { id: 'finder', name: 'Finder', icon: '📁', bg: 'linear-gradient(135deg, #007aff, #0051a8)' },
    { id: 'wordcraft', name: 'WordCraft', icon: '📝', bg: 'linear-gradient(135deg, #2563eb, #1e40af)' },
    { id: 'griidcraft', name: 'GridCraft', icon: '📊', bg: 'linear-gradient(135deg, #10b981, #047857)' },
    { id: 'deckcraft', name: 'DeckCraft', icon: '📽️', bg: 'linear-gradient(135deg, #f59e0b, #b45309)' },
    { id: 'notes', name: 'Notes', icon: '🗒️', bg: 'linear-gradient(135deg, #eab308, #ca8a04)' },
    { id: 'calendar', name: 'Calendar', icon: '📅', bg: 'linear-gradient(135deg, #ef4444, #b91c1c)' },
    { id: 'calculator', name: 'Calculator', icon: '🧮', bg: 'linear-gradient(135deg, #ff9f0a, #c2410c)' },
    { id: 'meet', name: 'BijiMeet', icon: '📹', bg: 'linear-gradient(135deg, #8b5cf6, #6d28d9)' },
    { id: 'pdfviewer', name: 'PDFCraft', icon: '📄', bg: 'linear-gradient(135deg, #dc2626, #991b1b)' },
    { id: 'gdrive', name: 'Google Drive', icon: '☁️', bg: 'linear-gradient(135deg, #38bdf8, #0284c7)' },
    { id: 'activity', name: 'Activity', icon: '📈', bg: 'linear-gradient(135deg, #14b8a6, #0f766e)' },
    { id: 'settings', name: 'Settings', icon: '⚙️', bg: 'linear-gradient(135deg, #64748b, #334155)' }
  ],

  isMobile() {
    return window.innerWidth <= 768;
  },

  init() {
    this.isStandalone =
      window.matchMedia('(display-mode: standalone)').matches ||
      window.navigator.standalone === true;

    this.registerServiceWorker();
    this.setupInstallPrompt();
    this.mountIosHomeScreen();
    this.patchDesktopForMobile();
    this.handleResize();

    window.addEventListener('resize', () => this.handleResize());

    // Handle PWA shortcut deep links e.g. /?app=meet
    const params = new URLSearchParams(window.location.search);
    const shortcutApp = params.get('app');
    if (shortcutApp && window.DokuHub) {
      setTimeout(() => {
        if (sessionStorage.getItem('dokuhub_unlocked') === 'true') {
          DokuHub.openApp(shortcutApp);
        }
      }, 650);
    }
  },

  registerServiceWorker() {
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker.register('/sw.js').catch((err) => {
        console.warn('PWA Service Worker registration skipped:', err);
      });
    }
  },

  setupInstallPrompt() {
    window.addEventListener('beforeinstallprompt', (e) => {
      e.preventDefault();
      this.deferredPrompt = e;
      this.updateInstallButton();
    });

    window.addEventListener('appinstalled', () => {
      this.deferredPrompt = null;
      this.isStandalone = true;
      this.updateInstallButton();
      const banner = document.getElementById('ios-pwa-banner');
      if (banner) banner.classList.add('hidden');
      if (window.DokuIsland) {
        DokuIsland.info('BijiOS berhasil dipasang ke Layar Utama!', 'PWA');
      }
    });
  },

  triggerInstall() {
    if (this.deferredPrompt) {
      this.deferredPrompt.prompt();
      this.deferredPrompt.userChoice.finally(() => {
        this.deferredPrompt = null;
        this.updateInstallButton();
      });
      return;
    }

    // iOS Safari manual guide
    const isIos = /iphone|ipad|ipod/i.test(navigator.userAgent);
    if (isIos) {
      this.showPwaBanner(
        '📱 Pasang BijiOS di iPhone/iPad: ketuk tombol <strong>Share (⬆️)</strong> di Safari lalu pilih <strong>Add to Home Screen</strong>.'
      );
    } else {
      this.showPwaBanner(
        '📱 Untuk memasang aplikasi: buka menu browser (⋮) lalu pilih <strong>Install app / Tambahkan ke Layar Utama</strong>.'
      );
    }
  },

  showPwaBanner(html) {
    let banner = document.getElementById('ios-pwa-banner');
    if (!banner) {
      banner = document.createElement('div');
      banner.id = 'ios-pwa-banner';
      banner.className = 'ios-pwa-banner';
      document.body.appendChild(banner);
    }
    banner.classList.remove('hidden');
    banner.innerHTML = `
      <div style="flex:1; line-height:1.4;">${html}</div>
      <button onclick="document.getElementById('ios-pwa-banner').classList.add('hidden')" style="background:rgba(255,255,255,0.18); border:none; color:#fff; width:26px; height:26px; border-radius:50%; font-weight:700; cursor:pointer; flex-shrink:0;">✕</button>
    `;
  },

  updateInstallButton() {
    const btn = document.getElementById('ios-install-btn');
    if (!btn) return;
    if (this.isStandalone) {
      btn.style.display = 'none';
    } else {
      btn.style.display = 'inline-flex';
    }
  },

  handleResize() {
    const mobile = this.isMobile();
    document.body.classList.toggle('is-mobile-view', mobile);
    if (mobile) {
      this.syncHomeScreenVisibility();
    }
  },

  mountIosHomeScreen() {
    if (document.getElementById('ios-home-screen')) return;
    const desktop = document.getElementById('macos-desktop') || document.body;

    const home = document.createElement('section');
    home.id = 'ios-home-screen';
    home.className = 'ios-home-screen';
    desktop.appendChild(home);
    this.renderIosHomeScreen();
  },

  renderIosHomeScreen() {
    const home = document.getElementById('ios-home-screen');
    if (!home) return;

    const user = (window.DokuHub && DokuHub.currentUser) || { name: 'User', role: 'Staff' };
    const tenant = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.name) || 'BijiOS Workspace';
    const runningIds = window.DokuHub ? Object.keys(DokuHub.windows || {}) : [];

    home.innerHTML = `
      <div class="ios-home-header">
        <div>
          <div class="ios-home-greeting">Halo, ${user.name}</div>
          <div class="ios-home-sub">${tenant} • ${user.role === 'Member' ? 'Staff' : user.role}</div>
        </div>
        <button id="ios-install-btn" class="ios-install-pill" onclick="DokuMobile.triggerInstall()" style="${this.isStandalone ? 'display:none;' : ''}">
          ⬇ Install App
        </button>
      </div>

      <div class="ios-app-grid">
        ${this.appCatalog.map((app) => {
          const isRunning = runningIds.includes(app.id);
          return `
            <div class="ios-app-tile ${isRunning ? 'running' : ''}" onclick="DokuHub.openApp('${app.id}')">
              <div class="ios-app-squircle" style="background:${app.bg};">
                <span>${app.icon}</span>
                <span class="ios-app-running-dot"></span>
              </div>
              <div class="ios-app-label">${app.name}</div>
            </div>
          `;
        }).join('')}
      </div>
    `;
  },

  goHome() {
    if (!window.DokuHub) return;
    Object.keys(DokuHub.windows || {}).forEach((appId) => {
      const win = DokuHub.windows[appId];
      if (win && win.el) {
        win.el.classList.add('minimized');
      }
    });
    this.syncHomeScreenVisibility();
  },

  syncHomeScreenVisibility() {
    const home = document.getElementById('ios-home-screen');
    if (!home) return;
    this.renderIosHomeScreen();

    if (!this.isMobile()) {
      home.classList.remove('has-active-app');
      return;
    }

    const wins = window.DokuHub ? Object.values(DokuHub.windows || {}) : [];
    const hasVisibleApp = wins.some((w) => w && w.el && !w.el.classList.contains('minimized'));
    home.classList.toggle('has-active-app', hasVisibleApp);
  },

  enhanceWindowTitlebar(winEl, appId) {
    const titlebar = winEl.querySelector('.mac-titlebar');
    if (!titlebar || titlebar.querySelector('.ios-nav-back')) return;

    const backBtn = document.createElement('button');
    backBtn.className = 'ios-nav-back';
    backBtn.type = 'button';
    backBtn.innerHTML = '‹ Home';
    backBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      this.goHome();
    });

    const closeBtn = document.createElement('button');
    closeBtn.className = 'ios-nav-close';
    closeBtn.type = 'button';
    closeBtn.title = 'Tutup Aplikasi';
    closeBtn.innerHTML = '✕';
    closeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      if (window.DokuHub) DokuHub.closeWindow(appId);
    });

    titlebar.insertBefore(backBtn, titlebar.firstChild);
    titlebar.appendChild(closeBtn);
  },

  patchDesktopForMobile() {
    if (!window.DokuHub || window._dkh_mobile_patched) return;
    window._dkh_mobile_patched = true;

    const origCreate = DokuHub.createWindow.bind(DokuHub);
    DokuHub.createWindow = (appId) => {
      const winObj = origCreate(appId);
      if (winObj && winObj.el) {
        this.enhanceWindowTitlebar(winObj.el, appId);
      }
      return winObj;
    };

    const origOpen = DokuHub.openApp.bind(DokuHub);
    DokuHub.openApp = (appId, initData = null) => {
      if (this.isMobile()) {
        // Minimize other windows so only the tapped app is visible fullscreen
        Object.keys(DokuHub.windows || {}).forEach((id) => {
          if (id !== appId && DokuHub.windows[id] && DokuHub.windows[id].el) {
            DokuHub.windows[id].el.classList.add('minimized');
          }
        });
      }
      origOpen(appId, initData);
      this.syncHomeScreenVisibility();
    };

    const origClose = DokuHub.closeWindow.bind(DokuHub);
    DokuHub.closeWindow = (appId) => {
      origClose(appId);
      setTimeout(() => this.syncHomeScreenVisibility(), 260);
    };

    const origMin = DokuHub.minimizeWindow.bind(DokuHub);
    DokuHub.minimizeWindow = (appId) => {
      origMin(appId);
      setTimeout(() => this.syncHomeScreenVisibility(), 260);
    };

    const origLaunchpad = DokuHub.toggleLaunchpad.bind(DokuHub);
    DokuHub.toggleLaunchpad = () => {
      if (this.isMobile()) {
        this.goHome();
        return;
      }
      origLaunchpad();
    };

    const origSwitchUser = DokuHub.switchUser.bind(DokuHub);
    DokuHub.switchUser = (user) => {
      origSwitchUser(user);
      this.renderIosHomeScreen();
    };
  }
};

window.addEventListener('DOMContentLoaded', () => {
  DokuMobile.init();
});
