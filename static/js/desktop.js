/**
 * DokuHub Desktop Window Manager & Core Shell
 * Provides macOS windowing, dock magnification, wallpaper switcher,
 * Spotlight search, and multi-tenant authentication.
 */

window.DokuHub = {
  activeTenant: { id: "", name: "Workspace", role: "Member" },
  currentUser: null,
  windows: {},
  topZIndex: 100,
  wallpapers: [
    { id: "sequoia", name: "macOS Sequoia Redwood", url: "https://images.unsplash.com/photo-1506744038136-46273834b3fb?auto=format&fit=crop&w=2560&q=80" },
    { id: "sonoma", name: "macOS Sonoma Valley", url: "https://images.unsplash.com/photo-1511497584788-87676104235f?auto=format&fit=crop&w=2560&q=80" },
    { id: "bigsur", name: "Big Sur Abstract Flow", url: "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?auto=format&fit=crop&w=2560&q=80" },
    { id: "cyber", name: "Cyber Gradient Night", url: "https://images.unsplash.com/photo-1550684848-fac1c5b4e853?auto=format&fit=crop&w=2560&q=80" },
    { id: "minimal", name: "Midnight Obsidian", url: "https://images.unsplash.com/photo-1579546929518-9e396f3cc809?auto=format&fit=crop&w=2560&q=80" }
  ],
  currentWallpaper: null,

  setupFetchInterceptor() {
    if (window._dkh_fetch_intercepted) return;
    window._dkh_fetch_intercepted = true;
    const originalFetch = window.fetch;
    window.fetch = async function (...args) {
      let [resource, config] = args;
      config = config || {};

      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) {
        if (!config.headers) {
          config.headers = { 'Authorization': `Bearer ${token}` };
        } else if (config.headers instanceof Headers) {
          if (!config.headers.has('Authorization')) config.headers.set('Authorization', `Bearer ${token}`);
          if (!config.headers.has('X-Tenant-Id') && DokuHub.currentUser && DokuHub.currentUser.tenant_id) {
            config.headers.set('X-Tenant-Id', DokuHub.currentUser.tenant_id);
          }
        } else if (Array.isArray(config.headers)) {
          config.headers.push(['Authorization', `Bearer ${token}`]);
          if (DokuHub.currentUser && DokuHub.currentUser.tenant_id) {
            config.headers.push(['X-Tenant-Id', DokuHub.currentUser.tenant_id]);
          }
        } else {
          if (!config.headers['Authorization']) config.headers['Authorization'] = `Bearer ${token}`;
          if (!config.headers['X-Tenant-Id'] && DokuHub.currentUser && DokuHub.currentUser.tenant_id) {
            config.headers['X-Tenant-Id'] = DokuHub.currentUser.tenant_id;
          }
        }
      }

      const response = await originalFetch(resource, config);
      if (response.status === 401 && typeof resource === 'string' && resource.startsWith('/api/') && !resource.startsWith('/api/auth/login')) {
        if (window.DokuTahoeLogin && !DokuTahoeLogin.isLocked) {
          console.warn('Session unauthorized (401). Redirecting to login.');
          sessionStorage.removeItem('dokuhub_unlocked');
          sessionStorage.removeItem('dokuhub_token');
          DokuTahoeLogin.lockScreen();
        }
      }
      return response;
    };
  },

  async init() {
    this.setupFetchInterceptor();

    const savedUser = sessionStorage.getItem('dokuhub_current_user') || localStorage.getItem('dokuhub_current_user');
    if (savedUser) {
      try {
        this.currentUser = JSON.parse(savedUser);
      } catch { }
    }

    if (this.currentUser) {
      this.switchUser(this.currentUser);
    }

    this.initClock();
    this.initWallpaper();
    this.initSpotlight();
    this.initTenantSelector();
    this.initTheme();
    this.initAccentColor();
    this.initDockPreferences();
    this.initDesktopContextMenu();
    if (window.DokuWidgets) {
      DokuWidgets.init();
    }

    // Auto launch File Manager on desktop start if unlocked
    const isUnlocked = sessionStorage.getItem('dokuhub_unlocked');
    if (isUnlocked === 'true') {
      setTimeout(() => {
        this.openApp("finder");
        if (window.DokuWelcome && this.currentUser) {
          DokuWelcome.checkAndTrigger(this.currentUser);
        }
      }, 300);
    }
  },

  switchUser(user) {
    if (!user) return;
    const initials = (user.name || 'User').split(' ').map(n => n[0]).join('').slice(0, 2).toUpperCase();
    const customAvatar = localStorage.getItem(`dokuhub_custom_avatar_${user.id}`);
    const customColor = localStorage.getItem(`dokuhub_custom_color_${user.id}`);

    const assignedTenant = user.tenant_name || user.tenant || (this.activeTenant ? this.activeTenant.name : 'Workspace');
    const assignedTenantId = user.tenant_id || (this.activeTenant ? this.activeTenant.id : '');

    const effectiveAvatar = customAvatar || user.avatar || initials;
    const effectiveColor = customColor || user.color || '#007aff';

    this.currentUser = {
      id: user.id || 'usr_user',
      name: user.name || 'User',
      email: user.email || '',
      role: user.role || 'Member',
      tenant_id: assignedTenantId,
      tenant: assignedTenant,
      avatar: effectiveAvatar,
      color: effectiveColor
    };

    if (assignedTenant) {
      this.activeTenant = {
        id: assignedTenantId,
        name: assignedTenant,
        role: this.currentUser.role
      };
    }

    sessionStorage.setItem('dokuhub_current_user', JSON.stringify(this.currentUser));
    localStorage.setItem('dokuhub_current_user', JSON.stringify(this.currentUser));

    const newToken = `dkh_token:${this.currentUser.id}:${this.currentUser.tenant_id}`;
    sessionStorage.setItem('dokuhub_token', newToken);
    localStorage.setItem('dokuhub_token', newToken);

    // Update active tenant name in menubar & window headers
    const activeTenantEl = document.getElementById('active-tenant-name');
    if (activeTenantEl) {
      activeTenantEl.textContent = this.currentUser.tenant;
    }
    document.querySelectorAll('.window-actions span').forEach(el => el.textContent = this.currentUser.tenant);

    // Update topbar user avatar and name
    const topAvatar = document.getElementById('topbar-user-avatar');
    const topName = document.getElementById('topbar-user-name');
    const curAvatar = this.currentUser.avatar || initials;
    const curColor = this.currentUser.color || '#007aff';
    const isImg = curAvatar && (
      curAvatar.startsWith('data:image') ||
      curAvatar.startsWith('/api/users/') ||
      curAvatar.startsWith('http://') ||
      curAvatar.startsWith('https://') ||
      curAvatar.includes('/avatar')
    );

    if (topAvatar) {
      if (isImg) {
        topAvatar.innerHTML = `<img src="${curAvatar}" style="width:100%; height:100%; border-radius:50%; object-fit:cover; display:block;" onerror="this.onerror=null; this.parentElement.style.background='${curColor}'; this.parentElement.textContent='${initials}';">`;
        topAvatar.style.background = 'transparent';
      } else {
        topAvatar.textContent = (curAvatar && curAvatar.length <= 4) ? curAvatar : initials;
        topAvatar.style.background = curColor;
      }
    }
    if (topName) {
      topName.textContent = this.currentUser.name;
    }

    const dropAvatarBadge = document.getElementById('dropdown-user-avatar-badge');
    if (dropAvatarBadge) {
      if (isImg) {
        dropAvatarBadge.innerHTML = `<img src="${curAvatar}" style="width:100%; height:100%; border-radius:50%; object-fit:cover; display:block;" onerror="this.onerror=null; this.parentElement.style.background='${curColor}'; this.parentElement.textContent='${initials}';">`;
        dropAvatarBadge.style.background = 'transparent';
      } else {
        dropAvatarBadge.textContent = (curAvatar && curAvatar.length <= 4) ? curAvatar : initials;
        dropAvatarBadge.style.background = curColor;
        dropAvatarBadge.style.display = 'flex';
        dropAvatarBadge.style.alignItems = 'center';
        dropAvatarBadge.style.justifyContent = 'center';
        dropAvatarBadge.style.fontWeight = '700';
        dropAvatarBadge.style.color = '#fff';
      }
    }

    const dropName = document.getElementById('dropdown-user-name');
    const dropEmail = document.getElementById('dropdown-user-email');
    const dropRole = document.getElementById('dropdown-user-role');
    if (dropName) dropName.textContent = this.currentUser.name;
    if (dropEmail) dropEmail.textContent = this.currentUser.email;
    if (dropRole) dropRole.textContent = `${this.currentUser.role} • ${this.currentUser.tenant}`;

    // Hide or show RBAC settings in menu based on role
    const rbacLink = document.getElementById('dropdown-menu-rbac-link');
    if (rbacLink) {
      rbacLink.style.display = (this.currentUser.role === 'SuperAdmin' || this.currentUser.role === 'Admin') ? 'flex' : 'none';
    }

    // Load and apply this member's personalized wallpaper preference
    const userWpKey = `dokuhub_wallpaper_${this.currentUser.id}`;
    const userWp = localStorage.getItem(userWpKey) || this.currentUser.wallpaper || localStorage.getItem('dokuhub_wallpaper') || this.wallpapers[0].url;
    this.setWallpaper(userWp, false);

    // Reload files in Finder if Finder is running
    if (window.DokuFinder && typeof window.DokuFinder.loadAllFiles === 'function') {
      window.DokuFinder.currentFolderId = 'root';
      window.DokuFinder.currentCategory = 'all';
      window.DokuFinder.selectedFileId = null;
      window.DokuFinder.loadAllFiles();
    }

    // Refresh Settings if running
    if (window.DokuSettings && DokuSettings.container) {
      DokuSettings.loadAllData().then(() => DokuSettings.render());
    }
  },

  async toggleUserMenu() {
    const menu = document.getElementById('user-menu-dropdown');
    if (!menu) return;
    menu.style.display = (menu.style.display === 'block') ? 'none' : 'block';
  },

  initClock() {
    const updateTime = () => {
      const now = new Date();
      const options = { weekday: 'short', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' };
      const lang = window.DokuI18n ? window.DokuI18n.lang : 'id';
      const locale = lang === 'en' ? 'en-US' : 'id-ID';
      const el = document.getElementById('menubar-clock');
      if (el) el.textContent = now.toLocaleDateString(locale, options);

      // Update live Calendar Dock icon month & day
      const calMonth = document.getElementById('dock-cal-month');
      const calDay = document.getElementById('dock-cal-day');
      if (calMonth) calMonth.textContent = now.toLocaleDateString(locale, { month: 'short' }).toUpperCase();
      if (calDay) calDay.textContent = String(now.getDate());
    };
    updateTime();
    setInterval(updateTime, 1000);
  },

  initDesktopContextMenu() {
    let menu = document.getElementById('desktop-context-menu');
    if (!menu) {
      menu = document.createElement('div');
      menu.id = 'desktop-context-menu';
      menu.className = 'desktop-context-menu';
      document.body.appendChild(menu);
    }

    document.addEventListener('contextmenu', (e) => {
      // If clicking inside a window, dock, or menubar, don't show desktop menu
      if (e.target.closest('.mac-window') || e.target.closest('.mac-dock-container') || e.target.closest('.mac-menubar') || e.target.closest('.modal') || e.target.closest('.widgets-modal-overlay')) {
        menu.style.display = 'none';
        return;
      }

      e.preventDefault();
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      const isWidgetVis = window.DokuWidgets ? DokuWidgets.isVisible : true;

      menu.innerHTML = `
        <div class="sidebar-item" onclick="if(window.DokuWidgets) DokuWidgets.openManageModal(); document.getElementById('desktop-context-menu').style.display='none';">
          ◫ ${isId ? 'Atur Widget Desktop...' : 'Edit Desktop Widgets...'}
        </div>
        <div class="sidebar-item" onclick="if(window.DokuWidgets) DokuWidgets.toggleVisibility(); document.getElementById('desktop-context-menu').style.display='none';">
          ${isWidgetVis ? (isId ? '👁️ Sembunyikan Widget' : '👁️ Hide Widgets') : (isId ? '👁️ Tampilkan Widget' : '👁️ Show Widgets')}
        </div>
        <div class="desktop-context-divider"></div>
        <div class="sidebar-item" onclick="DokuHub.openApp('wallpaper'); document.getElementById('desktop-context-menu').style.display='none';">
          🖼️ ${isId ? 'Ubah Wallpaper Desktop...' : 'Change Wallpaper...'}
        </div>
        <div class="sidebar-item" onclick="DokuHub.openApp('finder'); document.getElementById('desktop-context-menu').style.display='none';">
          📁 ${isId ? 'Buka Finder' : 'Open Finder'}
        </div>
        <div class="sidebar-item" onclick="DokuHub.openApp('settings'); document.getElementById('desktop-context-menu').style.display='none';">
          ⚙️ ${isId ? 'Pengaturan Sistem...' : 'System Settings...'}
        </div>
      `;

      let x = e.clientX;
      let y = e.clientY;
      if (x + 235 > window.innerWidth) x = window.innerWidth - 240;
      if (y + 210 > window.innerHeight) y = window.innerHeight - 215;

      menu.style.left = `${x}px`;
      menu.style.top = `${y}px`;
      menu.style.display = 'none';
      void menu.offsetWidth; // re-trigger pop-in animation
      menu.style.display = 'block';
    });

    document.addEventListener('click', (e) => {
      if (!e.target.closest('#desktop-context-menu')) {
        menu.style.display = 'none';
      }
    });
  },

  getUserWallpaperKey() {
    return this.currentUser ? `dokuhub_wallpaper_${this.currentUser.id}` : 'dokuhub_wallpaper';
  },

  initWallpaper() {
    const key = this.getUserWallpaperKey();
    const saved = localStorage.getItem(key) || localStorage.getItem('dokuhub_wallpaper') || this.wallpapers[0].url;
    this.setWallpaper(saved, false);
  },

  setWallpaper(url, persist = true) {
    if (!url) return;
    this.currentWallpaper = url;
    document.body.style.backgroundImage = `url('${url}')`;
    if (persist) {
      const key = this.getUserWallpaperKey();
      localStorage.setItem(key, url);
      localStorage.setItem('dokuhub_wallpaper', url);
      if (this.currentUser) {
        this.currentUser.wallpaper = url;
        sessionStorage.setItem('dokuhub_current_user', JSON.stringify(this.currentUser));
        localStorage.setItem('dokuhub_current_user', JSON.stringify(this.currentUser));
      }
    }
  },

  initTheme() {
    const savedTheme = localStorage.getItem('dokuhub_theme') || 'light';
    document.documentElement.setAttribute('data-theme', savedTheme);
  },

  toggleTheme() {
    const cur = document.documentElement.getAttribute('data-theme') || 'light';
    const next = cur === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    localStorage.setItem('dokuhub_theme', next);
  },

  initAccentColor() {
    const color = localStorage.getItem('dokuhub_accent_color') || '#007aff';
    document.documentElement.style.setProperty('--accent-color', color);
  },

  setAccentColor(color) {
    document.documentElement.style.setProperty('--accent-color', color);
    localStorage.setItem('dokuhub_accent_color', color);
  },

  initDockPreferences() {
    const size = localStorage.getItem('dokuhub_dock_size') || '52';
    const magnify = localStorage.getItem('dokuhub_dock_magnify') !== 'false';
    const pos = localStorage.getItem('dokuhub_dock_position') || 'bottom';

    document.documentElement.style.setProperty('--dock-item-size', `${size}px`);
    document.documentElement.style.setProperty('--dock-hover-scale', magnify ? '1.18' : '1.0');

    const dockContainer = document.querySelector('.mac-dock-container');
    if (dockContainer) {
      dockContainer.classList.remove('dock-left', 'dock-right');
      if (pos === 'left') dockContainer.classList.add('dock-left');
      if (pos === 'right') dockContainer.classList.add('dock-right');
    }
  },

  async initTenantSelector() {
    const pill = document.getElementById('tenant-switcher-pill');
    if (pill) {
      pill.addEventListener('click', () => {
        this.openApp('settings');
        if (window.DokuSettings) {
          window.DokuSettings.showTab('tenants');
        }
      });
    }

    // Dynamically fetch and sync active tenant from database
    try {
      const res = await fetch('/api/tenants');
      const data = await res.json();
      if (res.ok && Array.isArray(data.tenants) && data.tenants.length > 0) {
        let matched = null;
        if (this.currentUser && this.currentUser.tenant_id) {
          matched = data.tenants.find(t => t.id === this.currentUser.tenant_id);
        }
        if (!matched) {
          matched = data.tenants[0];
        }
        if (matched) {
          this.activeTenant = {
            id: matched.id,
            name: matched.name,
            role: this.currentUser ? this.currentUser.role : 'SuperAdmin'
          };
          if (this.currentUser) {
            this.currentUser.tenant_id = matched.id;
            this.currentUser.tenant = matched.name;
            sessionStorage.setItem('dokuhub_current_user', JSON.stringify(this.currentUser));
            localStorage.setItem('dokuhub_current_user', JSON.stringify(this.currentUser));
          }
          const activeTenantEl = document.getElementById('active-tenant-name');
          if (activeTenantEl) activeTenantEl.textContent = matched.name;
          document.querySelectorAll('.window-actions span').forEach(el => el.textContent = matched.name);
        }
      }
    } catch (_) { }
  },

  initSpotlight() {
    window.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        this.toggleSpotlight();
      }
      if (e.key === 'Escape') {
        const spot = document.getElementById('spotlight-modal');
        if (spot && spot.style.display === 'flex') {
          spot.style.display = 'none';
        }
        const launchpad = document.getElementById('launchpad-modal');
        if (launchpad && launchpad.style.display === 'flex') {
          this.toggleLaunchpad(false);
        }
      }
    });

    const spotInput = document.getElementById('spotlight-input');
    if (spotInput) {
      spotInput.addEventListener('input', (e) => this.handleSpotlightSearch(e.target.value));
    }
  },

  toggleSpotlight() {
    const spot = document.getElementById('spotlight-modal');
    if (!spot) return;
    if (spot.style.display === 'flex') {
      spot.style.display = 'none';
    } else {
      spot.style.display = 'flex';
      const input = document.getElementById('spotlight-input');
      if (input) {
        input.value = '';
        input.focus();
        this.handleSpotlightSearch('');
      }
    }
  },

  handleSpotlightSearch(query) {
    const container = document.getElementById('spotlight-results');
    if (!container) return;

    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const apps = [
      { id: 'finder', name: 'Finder', desc: isId ? 'Kelola berkas & bagikan tautan internal / publik' : 'Manage files, storage & share links', icon: '📁' },
      { id: 'wordcraft', name: 'WordCraft', desc: isId ? 'Editor dokumen Word clean-room buatan sendiri' : 'Clean-room Word document processor', icon: '📝' },
      { id: 'griidcraft', name: 'GriidCraft', desc: isId ? 'Spreadsheet canggih dengan formula real-time' : 'Spreadsheet with real-time math formulas', icon: '📊' },
      { id: 'deckcraft', name: 'DeckCraft', desc: isId ? 'Presentasi interaktif & slideshow mode' : 'Interactive presentation slides', icon: '📽️' },
      { id: 'pdfviewer', name: 'PDFCraft', desc: isId ? 'Baca, edit, anotasi, tanda tangan, merge & split PDF' : 'Read, edit, annotate, sign, merge & split PDF documents', icon: '📄' },
      { id: 'notes', name: 'Notes & Markdown', desc: isId ? 'Catatan & dokumen Markdown dengan Auto Preview' : 'Notes & Markdown editor with live Auto Preview', icon: '🗒️' },
      { id: 'calendar', name: isId ? 'Kalender (Calendar)' : 'Calendar', desc: isId ? 'Jadwal, agenda bulanan & pengingat acara' : 'Monthly schedule, events & reminders', icon: '📅' },
      { id: 'calculator', name: isId ? 'Kalkulator (Calculator)' : 'Calculator', desc: isId ? 'Kalkulator standar & ilmiah dengan riwayat' : 'Standard & scientific calculator with history', icon: '🧮' },
      { id: 'meet', name: 'BijiMeet', desc: isId ? 'Video conference WebRTC self-hosted' : 'Self-hosted WebRTC video conferencing', icon: '🎥' },
      { id: 'gdrive', name: 'Google Drive Connector', desc: isId ? 'Sinkronisasi berkas Google Drive' : 'Sync and import Google Drive files', icon: '☁️' },
      { id: 'settings', name: 'Settings', desc: isId ? 'RBAC, profil, OpenAPI & Database TOON v4' : 'RBAC, profile, OpenAPI docs & TOON v4 DB', icon: '⚙️' },
      { id: 'wallpaper', name: 'Change Wallpaper', desc: isId ? 'Ubah latar belakang desktop macOS' : 'Change macOS desktop wallpaper background', icon: '🖼️' },
      { id: 'about', name: isId ? 'Tentang BijiOS' : 'About This BijiOS', desc: isId ? 'Spesifikasi sistem, kernel Rust, arsitektur & penyimpanan' : 'System specs, Rust kernel, TOON architecture & storage', icon: '🍏' },
      { id: 'activity', name: isId ? 'Monitor Aktivitas (Task Manager)' : 'Activity Monitor (Task Manager)', desc: isId ? 'Pantau CPU, Memori, Storage, Disk & Proses secara real-time' : 'Real-time CPU, Memory, Storage, Disk & Process monitor', icon: '📈' },
      { id: 'preview', name: isId ? 'Pratinjau Gambar (Preview)' : 'Preview (Image Viewer)', desc: isId ? 'Lihat gambar JPG, PNG, BMP & jadikan wallpaper desktop' : 'View JPG, PNG, BMP images & set desktop wallpaper', icon: '🖼️' }
    ];

    const q = query.toLowerCase().trim();
    const matches = apps.filter(a => a.name.toLowerCase().includes(q) || a.desc.toLowerCase().includes(q));

    container.innerHTML = matches.map((item, idx) => `
      <div class="spotlight-result-item ${idx === 0 ? 'highlighted' : ''}" onclick="DokuHub.openApp('${item.id}'); DokuHub.toggleSpotlight();">
        <span style="font-size:24px;">${item.icon}</span>
        <div>
          <div style="font-weight:600; font-size:14px;">${item.name}</div>
          <div style="font-size:12px; opacity:0.8;">${item.desc}</div>
        </div>
      </div>
    `).join('');
  },

  /**
   * macOS Launchpad (All Applications Drawer)
   */
  toggleLaunchpad(forceState = null) {
    const modal = document.getElementById('launchpad-modal');
    if (!modal) return;
    const shouldOpen = forceState !== null ? forceState : (modal.style.display !== 'flex');

    if (shouldOpen) {
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      const input = document.getElementById('launchpad-search-input');
      if (input) {
        input.placeholder = isId ? 'Cari Aplikasi...' : 'Search Applications...';
        input.value = '';
      }
      this.renderLaunchpadGrid('');
      modal.style.display = 'flex';
      if (input) setTimeout(() => input.focus(), 60);
    } else {
      modal.style.display = 'none';
    }
  },

  renderLaunchpadGrid(query = '') {
    const grid = document.getElementById('launchpad-grid');
    if (!grid) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const q = (query || '').toLowerCase().trim();

    const catalog = [
      { id: 'finder', name: 'Finder', icon: '📁', bg: 'linear-gradient(135deg, #007aff 0%, #0051a8 100%)' },
      { id: 'wordcraft', name: 'WordCraft', icon: '📝', bg: 'linear-gradient(135deg, #2563eb 0%, #1e40af 100%)' },
      { id: 'griidcraft', name: 'GridCraft', icon: '📊', bg: 'linear-gradient(135deg, #10b981 0%, #047857 100%)' },
      { id: 'deckcraft', name: 'DeckCraft', icon: '📽️', bg: 'linear-gradient(135deg, #f59e0b 0%, #b45309 100%)' },
      { id: 'pdfviewer', name: 'PDFCraft', icon: '📄', bg: 'linear-gradient(135deg, #ef4444 0%, #b91c1c 100%)' },
      { id: 'notes', name: isId ? 'Notes & Markdown' : 'Notes & Markdown', icon: '🗒️', bg: 'linear-gradient(135deg, #fbbf24 0%, #d97706 100%)' },
      { id: 'calendar', name: isId ? 'Kalender' : 'Calendar', icon: '📅', bg: 'linear-gradient(135deg, #ff3b30 0%, #d70015 100%)' },
      { id: 'calculator', name: isId ? 'Kalkulator' : 'Calculator', icon: '🧮', bg: 'linear-gradient(135deg, #ff9f0a 0%, #c2410c 100%)' },
      { id: 'meet', name: 'BijiMeet', icon: '🎥', bg: 'linear-gradient(135deg, #8b5cf6 0%, #6d28d9 100%)' },
      { id: 'gdrive', name: 'Google Drive', icon: '☁️', bg: 'linear-gradient(135deg, #38bdf8 0%, #0284c7 100%)' },
      { id: 'activity', name: isId ? 'Monitor Aktivitas' : 'Activity Monitor', icon: '📈', bg: 'linear-gradient(135deg, #10b981 0%, #059669 100%)' },
      { id: 'wallpaper', name: 'Wallpaper', icon: '🖼️', bg: 'linear-gradient(135deg, #ec4899 0%, #be185d 100%)' },
      { id: 'settings', name: isId ? 'Pengaturan' : 'Settings', icon: '⚙️', bg: 'linear-gradient(135deg, #64748b 0%, #334155 100%)' },
      { id: 'about', name: isId ? 'Tentang BijiOS' : 'About BijiOS', icon: '🍏', bg: 'linear-gradient(135deg, #1e293b 0%, #0f172a 100%)' }
    ];

    const filtered = catalog.filter(a => a.name.toLowerCase().includes(q) || a.id.includes(q));
    let html = filtered.map(a => `
      <div class="launchpad-app-item" onclick="DokuHub.toggleLaunchpad(false); DokuHub.openApp('${a.id}');">
        <div class="launchpad-app-icon" style="background:${a.bg};">${a.icon}</div>
        <div class="launchpad-app-label">${a.name}</div>
      </div>
    `).join('');

    // Append installed native Linux (.deb) applications when available
    if (this._nativeAppsLoaded === undefined) {
      this._nativeAppsLoaded = true;
      this.loadNativeLinuxApps();
    }
    const nativeList = (this.nativeApps || []).filter(a =>
      a.name.toLowerCase().includes(q) || (a.category || '').toLowerCase().includes(q) || a.id.toLowerCase().includes(q)
    );
    if (nativeList.length > 0) {
      html += nativeList.map(a => `
        <div class="launchpad-app-item" title="${(a.comment || a.category || 'Linux App').replace(/"/g, '&quot;')}"
          onclick="DokuHub.toggleLaunchpad(false); DokuHub.launchNativeApp('${a.id.replace(/'/g, "\\'")}', '${a.name.replace(/'/g, "\\'")}');">
          <div class="launchpad-app-icon" style="background:linear-gradient(135deg, #334155 0%, #0f172a 100%); position:relative;">
            ${a.icon || '📦'}
            <span style="position:absolute; bottom:-3px; right:-3px; font-size:9px; background:#0ea5e9; color:#fff; padding:1px 4px; border-radius:6px; font-weight:700;">DEB</span>
          </div>
          <div class="launchpad-app-label">${a.name}</div>
        </div>
      `).join('');
    }
    grid.innerHTML = html;
  },

  async loadNativeLinuxApps() {
    try {
      const res = await fetch('/api/os/apps');
      if (res.ok) {
        const data = await res.json();
        this.nativeApps = data.apps || [];
        const modal = document.getElementById('launchpad-modal');
        if (modal && modal.style.display === 'flex') {
          const q = document.getElementById('launchpad-search-input')?.value || '';
          this.renderLaunchpadGrid(q);
        }
      }
    } catch { }
  },

  async launchNativeApp(id, name) {
    try {
      const res = await fetch('/api/os/launch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id })
      });
      const data = await res.json();
      if (data.success && window.DokuIsland) {
        DokuIsland.info(`Menjalankan aplikasi Linux: ${name}`, 'Desktop OS');
      } else if (!data.success) {
        alert('Gagal menjalankan ' + name + ': ' + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert('Error: ' + e);
    }
  },

  async osPower(action) {
    const labels = { suspend: 'Sleep / Suspend', restart: 'Restart komputer', poweroff: 'Matikan (Shut Down) komputer' };
    if (!confirm(`Yakin ingin ${labels[action] || action}?`)) return;
    try {
      const res = await fetch('/api/os/power', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action })
      });
      if (!res.ok) {
        if (action === 'suspend' && window.DokuTahoeLogin) DokuTahoeLogin.sleepSystem();
        else if (action === 'restart' && window.DokuTahoeLogin) DokuTahoeLogin.restartSystem();
      }
    } catch {
      if (action === 'suspend' && window.DokuTahoeLogin) DokuTahoeLogin.sleepSystem();
      else if (action === 'restart' && window.DokuTahoeLogin) DokuTahoeLogin.restartSystem();
    }
  },

  ensureDynamicDockIcon(appId) {
    let dockItem = document.querySelector(`.dock-item[data-app="${appId}"]`);
    if (dockItem) return dockItem;

    const dynContainer = document.getElementById('dock-dynamic-apps');
    const dynDivider = document.getElementById('dock-dynamic-divider');
    if (!dynContainer) return null;

    const cfg = this.getAppConfig(appId);
    const bgMap = {
      deckcraft: 'linear-gradient(135deg, #f59e0b 0%, #b45309 100%)',
      pdfviewer: 'linear-gradient(135deg, #ef4444 0%, #b91c1c 100%)',
      calculator: 'linear-gradient(135deg, #ff9f0a 0%, #c2410c 100%)',
      gdrive: 'linear-gradient(135deg, #38bdf8 0%, #0284c7 100%)',
      activity: 'linear-gradient(135deg, #10b981 0%, #059669 100%)',
      wallpaper: 'linear-gradient(135deg, #ec4899 0%, #be185d 100%)',
      about: 'linear-gradient(135deg, #1e293b 0%, #0f172a 100%)',
      preview: 'linear-gradient(135deg, #06b6d4 0%, #0369a1 100%)'
    };

    dockItem = document.createElement('div');
    dockItem.className = 'dock-item dynamic-dock-item running';
    dockItem.setAttribute('data-app', appId);
    dockItem.onclick = () => this.openApp(appId);
    dockItem.innerHTML = `
      <div class="dock-tooltip">${cfg.title}</div>
      <div class="dock-icon" style="background:${bgMap[appId] || 'linear-gradient(135deg, #64748b, #334155)'}; color:#fff;">${cfg.icon}</div>
      <div class="dock-dot"></div>
    `;
    dynContainer.appendChild(dockItem);
    if (dynDivider) dynDivider.style.display = 'block';
    return dockItem;
  },

  removeDynamicDockIcon(appId) {
    const dynItem = document.querySelector(`#dock-dynamic-apps .dock-item[data-app="${appId}"]`);
    if (dynItem) dynItem.remove();
    const dynContainer = document.getElementById('dock-dynamic-apps');
    const dynDivider = document.getElementById('dock-dynamic-divider');
    if (dynDivider && dynContainer && dynContainer.children.length === 0) {
      dynDivider.style.display = 'none';
    }
  },

  logAudit(category, action, detail, resource = 'desktop', severity = 'info') {
    try {
      const headers = { 'Content-Type': 'application/json' };
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      if (this.currentUser && this.currentUser.tenant_id) headers['X-Tenant-Id'] = this.currentUser.tenant_id;
      fetch('/api/audit/log', {
        method: 'POST',
        headers,
        body: JSON.stringify({ category, action, detail, resource, severity })
      }).catch(() => { });
    } catch { }
  },

  /**
   * Floating Window Management
   */
  openApp(appId, initData = null) {
    let win = this.windows[appId];
    let isNew = false;

    // Trigger macOS Dock icon bounce (or spawn temporary Dock icon for unpinned apps)
    const dockItem = this.ensureDynamicDockIcon(appId);
    if (dockItem) {
      dockItem.classList.add('running');
      dockItem.classList.remove('bouncing');
      void dockItem.offsetWidth;
      dockItem.classList.add('bouncing');
      setTimeout(() => dockItem.classList.remove('bouncing'), 560);
    }

    if (!win) {
      win = this.createWindow(appId);
      this.windows[appId] = win;
      isNew = true;

      // Record app launch in Security Audit Trail
      const cfg = this.getAppConfig(appId);
      this.logAudit('DESKTOP', 'APP_OPEN', `Membuka aplikasi ${cfg.title}`, `app:${appId}`, 'info');

      // Fluid spring open animation
      win.el.classList.add('opening');
      setTimeout(() => win.el.classList.remove('opening'), 320);
    } else if (win.el.classList.contains('minimized')) {
      // Fluid restore from Dock animation
      win.el.classList.remove('minimized', 'minimizing');
      win.el.classList.add('restoring');
      setTimeout(() => win.el.classList.remove('restoring'), 340);
    }

    this.bringToFront(win);

    // Notify app module if available (only re-init when new or explicitly passing initData)
    if (isNew || initData !== null) {
      this.triggerAppInit(appId, win.contentEl, initData);
    }
  },

  createWindow(appId) {
    const config = this.getAppConfig(appId);
    const space = document.getElementById('window-space');

    const winEl = document.createElement('div');
    winEl.className = 'mac-window';
    winEl.id = `window-${appId}`;
    winEl.style.width = `${config.defaultWidth}px`;
    winEl.style.height = `${config.defaultHeight}px`;
    if (config.minWidth) winEl.style.minWidth = `${config.minWidth}px`;

    // Smart cascade positioning
    const offset = Object.keys(this.windows).length * 28;
    const initialLeft = Math.max(30, Math.min(window.innerWidth - config.defaultWidth - 40, 60 + offset));
    const initialTop = Math.max(20, Math.min(window.innerHeight - config.defaultHeight - 80, 40 + offset));
    winEl.style.left = `${initialLeft}px`;
    winEl.style.top = `${initialTop}px`;

    winEl.innerHTML = `
      <div class="mac-titlebar">
        <div class="traffic-lights">
          <button class="traffic-btn btn-close" title="${window.DokuI18n && DokuI18n.lang === 'id' ? 'Tutup' : 'Close'}">
            <svg width="6" height="6" viewBox="0 0 6 6"><path d="M1 1l4 4m0-4L1 5" stroke="#4c0000" stroke-width="1.2" stroke-linecap="round"/></svg>
          </button>
          <button class="traffic-btn btn-min" title="${window.DokuI18n && DokuI18n.lang === 'id' ? 'Kecilkan ke Dock' : 'Minimize'}">
            <svg width="6" height="6" viewBox="0 0 6 6"><path d="M1 3h4" stroke="#5c3f00" stroke-width="1.2" stroke-linecap="round"/></svg>
          </button>
          <button class="traffic-btn btn-max" title="${window.DokuI18n && DokuI18n.lang === 'id' ? 'Perbesar' : 'Zoom'}">
            <svg width="6" height="6" viewBox="0 0 6 6"><path d="M1 1h4v4H1z" fill="#004d00"/></svg>
          </button>
        </div>
        <div class="window-title">${config.icon} ${config.title}</div>
        <div class="window-actions">
          <span style="font-size:11px; color:var(--text-muted);">${this.activeTenant.name}</span>
        </div>
      </div>
      <div class="mac-body" id="body-${appId}">
        <!-- App Content injected here -->
      </div>
      <!-- Resize handles -->
      <div class="resize-handle n"></div>
      <div class="resize-handle s"></div>
      <div class="resize-handle e"></div>
      <div class="resize-handle w"></div>
      <div class="resize-handle nw"></div>
      <div class="resize-handle ne"></div>
      <div class="resize-handle sw"></div>
      <div class="resize-handle se"></div>
    `;

    space.appendChild(winEl);

    const winObj = {
      appId,
      el: winEl,
      contentEl: winEl.querySelector(`#body-${appId}`),
      isMaximized: false,
      prevBounds: null
    };

    // Attach Titlebar Dragging
    const titlebar = winEl.querySelector('.mac-titlebar');
    this.makeDraggable(winEl, titlebar);

    // Attach Resizing
    this.makeResizable(winEl);

    // Attach Traffic Lights
    winEl.querySelector('.btn-close').addEventListener('click', () => this.closeWindow(appId));
    winEl.querySelector('.btn-min').addEventListener('click', () => this.minimizeWindow(appId));
    winEl.querySelector('.btn-max').addEventListener('click', () => this.maximizeWindow(appId));

    // Focus on click
    winEl.addEventListener('mousedown', () => this.bringToFront(winObj));

    return winObj;
  },

  makeDraggable(winEl, handle) {
    let isDragging = false;
    let startX, startY, origX, origY;

    handle.addEventListener('mousedown', (e) => {
      if (e.target.closest('.traffic-btn')) return;
      isDragging = true;
      startX = e.clientX;
      startY = e.clientY;
      origX = winEl.offsetLeft;
      origY = winEl.offsetTop;

      const onMouseMove = (ev) => {
        if (!isDragging) return;
        const dx = ev.clientX - startX;
        const dy = ev.clientY - startY;
        const newLeft = Math.max(0, Math.min(window.innerWidth - 80, origX + dx));
        const newTop = Math.max(0, Math.min(window.innerHeight - 80, origY + dy));
        winEl.style.left = `${newLeft}px`;
        winEl.style.top = `${newTop}px`;
      };

      const onMouseUp = () => {
        isDragging = false;
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    });
  },

  makeResizable(winEl) {
    const handles = winEl.querySelectorAll('.resize-handle');
    handles.forEach(handle => {
      handle.addEventListener('mousedown', (e) => {
        e.preventDefault();
        e.stopPropagation();
        const dir = handle.className.replace('resize-handle ', '');
        const startX = e.clientX;
        const startY = e.clientY;
        const startW = winEl.offsetWidth;
        const startH = winEl.offsetHeight;
        const startL = winEl.offsetLeft;
        const startT = winEl.offsetTop;

        const onMouseMove = (ev) => {
          const dx = ev.clientX - startX;
          const dy = ev.clientY - startY;

          if (dir.includes('e')) winEl.style.width = `${Math.max(420, startW + dx)}px`;
          if (dir.includes('s')) winEl.style.height = `${Math.max(280, startH + dy)}px`;
          if (dir.includes('w')) {
            const newW = Math.max(420, startW - dx);
            if (newW > 420) {
              winEl.style.width = `${newW}px`;
              winEl.style.left = `${startL + dx}px`;
            }
          }
          if (dir.includes('n')) {
            const newH = Math.max(280, startH - dy);
            if (newH > 280) {
              winEl.style.height = `${newH}px`;
              winEl.style.top = `${startT + dy}px`;
            }
          }
        };

        const onMouseUp = () => {
          window.removeEventListener('mousemove', onMouseMove);
          window.removeEventListener('mouseup', onMouseUp);
        };

        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
      });
    });
  },

  bringToFront(winObj) {
    this.topZIndex += 1;
    winObj.el.style.zIndex = this.topZIndex;
    this.activeAppId = winObj.appId;

    document.querySelectorAll('.mac-window').forEach(w => w.classList.remove('active-window'));
    winObj.el.classList.add('active-window');

    // Update Top Menu Active App
    const cfg = this.getAppConfig(winObj.appId);
    const appTitleMenu = document.getElementById('active-app-name');
    if (appTitleMenu) appTitleMenu.textContent = cfg.title;
  },

  minimizeWindow(appId) {
    const win = this.windows[appId];
    if (!win || win.el.classList.contains('minimizing') || win.el.classList.contains('minimized')) return;

    win.el.classList.remove('opening', 'restoring');
    win.el.classList.add('minimizing');
    setTimeout(() => {
      if (win.el.classList.contains('minimizing')) {
        win.el.classList.add('minimized');
        win.el.classList.remove('minimizing');
      }
    }, 270);
  },

  maximizeWindow(appId) {
    const win = this.windows[appId];
    if (!win) return;

    // Enable smooth geometry transition only during maximize/restore
    win.el.classList.add('maximizing-transition');

    if (!win.isMaximized) {
      win.prevBounds = {
        left: win.el.style.left,
        top: win.el.style.top,
        width: win.el.style.width,
        height: win.el.style.height
      };
      win.el.style.left = '4px';
      win.el.style.top = '4px';
      win.el.style.width = 'calc(100vw - 8px)';
      win.el.style.height = 'calc(100vh - 100px)';
      win.el.style.borderRadius = '8px';
      win.isMaximized = true;
    } else {
      if (win.prevBounds) {
        win.el.style.left = win.prevBounds.left;
        win.el.style.top = win.prevBounds.top;
        win.el.style.width = win.prevBounds.width;
        win.el.style.height = win.prevBounds.height;
      }
      win.el.style.borderRadius = '12px';
      win.isMaximized = false;
    }

    setTimeout(() => {
      if (win.el) win.el.classList.remove('maximizing-transition');
    }, 340);
  },

  closeWindow(appId) {
    const win = this.windows[appId];
    if (!win || win.el.classList.contains('closing')) return;

    // If closing DokuMeet, cleanly release camera/mic streams & leave room
    if (appId === 'meet' && window.DokuMeet) {
      if (DokuMeet.roomCode) {
        DokuMeet.sendSignal('leave', 'leave_room');
        DokuMeet.roomCode = null;
      }
      if (DokuMeet.pollTimer) {
        clearInterval(DokuMeet.pollTimer);
        DokuMeet.pollTimer = null;
      }
      DokuMeet.cleanupMedia();
    }

    win.el.classList.remove('opening', 'restoring');
    win.el.classList.add('closing');

    const dockItem = document.querySelector(`.dock-item[data-app="${appId}"]`);
    if (dockItem) dockItem.classList.remove('running');
    this.removeDynamicDockIcon(appId);

    setTimeout(() => {
      win.el.remove();
      delete this.windows[appId];
    }, 200);
  },

  getAppConfig(appId) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const map = {
      finder: { title: 'Finder', icon: '📁', defaultWidth: 920, defaultHeight: 560 },
      wordcraft: { title: 'WordCraft', icon: '📝', defaultWidth: 960, defaultHeight: 640 },
      griidcraft: { title: 'GridCraft', icon: '📊', defaultWidth: 1040, defaultHeight: 640 },
      deckcraft: { title: 'DeckCraft', icon: '📽️', defaultWidth: 1040, defaultHeight: 640 },
      pdfviewer: { title: 'PDFCraft', icon: '📄', defaultWidth: 1040, defaultHeight: 660 },
      notes: { title: 'Notes & Markdown', icon: '🗒️', defaultWidth: 980, defaultHeight: 600 },
      calendar: { title: isId ? 'Kalender' : 'Calendar', icon: '📅', defaultWidth: 920, defaultHeight: 600 },
      calculator: { title: isId ? 'Kalkulator' : 'Calculator', icon: '🧮', defaultWidth: 320, defaultHeight: 470, minWidth: 300 },
      meet: { title: 'BijiMeet', icon: '🎥', defaultWidth: 920, defaultHeight: 580 },
      gdrive: { title: 'Google Drive Connector', icon: '☁️', defaultWidth: 820, defaultHeight: 520 },
      settings: { title: 'Settings', icon: '⚙️', defaultWidth: 900, defaultHeight: 580 },
      wallpaper: { title: 'Wallpaper Chooser', icon: '🖼️', defaultWidth: 700, defaultHeight: 460 },
      about: { title: isId ? 'Tentang BijiOS' : 'About This BijiOS', icon: '🍏', defaultWidth: 640, defaultHeight: 560 },
      activity: { title: isId ? 'Monitor Aktivitas' : 'Activity Monitor', icon: '📈', defaultWidth: 960, defaultHeight: 620 },
      preview: { title: isId ? 'Pratinjau Gambar' : 'Preview', icon: '🖼️', defaultWidth: 840, defaultHeight: 600 }
    };
    return map[appId] || { title: 'Application', icon: '💻', defaultWidth: 700, defaultHeight: 500 };
  },

  triggerAppInit(appId, containerEl, initData) {
    if (appId === 'about' && window.DokuAbout) window.DokuAbout.init(containerEl, initData);
    if (appId === 'activity' && window.DokuActivity) window.DokuActivity.init(containerEl, initData);
    if (appId === 'preview' && window.DokuPreview) window.DokuPreview.init(containerEl, initData);
    if (appId === 'finder' && window.DokuFinder) window.DokuFinder.init(containerEl, initData);
    if (appId === 'wordcraft' && window.DokuWordCraft) window.DokuWordCraft.init(containerEl, initData);
    if (appId === 'griidcraft' && window.DokuGriidCraft) window.DokuGriidCraft.init(containerEl, initData);
    if (appId === 'deckcraft' && window.DokuDeckCraft) window.DokuDeckCraft.init(containerEl, initData);
    if (appId === 'pdfviewer' && window.DokuPdfViewer) window.DokuPdfViewer.init(containerEl, initData);
    if (appId === 'notes' && window.DokuNotes) window.DokuNotes.init(containerEl, initData);
    if (appId === 'calendar' && window.DokuCalendar) window.DokuCalendar.init(containerEl, initData);
    if (appId === 'calculator' && window.DokuCalculator) window.DokuCalculator.init(containerEl, initData);
    if (appId === 'meet' && window.DokuMeet) window.DokuMeet.init(containerEl, initData);
    if (appId === 'gdrive' && window.DokuGDrive) window.DokuGDrive.init(containerEl, initData);
    if (appId === 'settings' && window.DokuSettings) window.DokuSettings.init(containerEl, initData);
    if (appId === 'wallpaper' && window.DokuWallpaper) window.DokuWallpaper.init(containerEl, initData);
  }
};

window.addEventListener('DOMContentLoaded', () => {
  window.DokuHub.init();
});
