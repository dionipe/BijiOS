/**
 * DokuHub i18n (Internationalization) Engine
 * Dual Language Support: Indonesian (ID) and English (EN)
 */

window.DokuI18n = {
  lang: localStorage.getItem('dokuhub_lang') || 'en',

  translations: {
    id: {
      // Menu Bar & Apple Menu
      app_name: "Finder",
      file_menu: "Berkas",
      edit_menu: "Edit",
      view_menu: "Lihat",
      go_menu: "Buka",
      window_menu: "Jendela",
      help_menu: "Bantuan",
      about_dokuhub: "Tentang BijiOS Ini",
      activity_monitor: "Monitor Aktivitas...",
      sys_settings: "Pengaturan Sistem...",
      change_wallpaper: "Ubah Wallpaper Desktop...",
      lock_screen: "Kunci Layar",
      sleep_mode: "Tidur (Sleep)",
      restart_system: "Mulai Ulang...",
      logout_session: "Keluar Sesi",

      // Topbar User Menu
      user_profile: "👤 Profil & Kata Sandi...",
      user_appearance: "🎨 Tampilan & Ukuran Dock...",
      user_rbac: "👥 Manajemen Pengguna & RBAC...",
      user_lock: "🔒 Kunci Layar",
      user_logout: "🚪 Keluar Sesi",

      // Finder Sidebar
      storage_section: "Penyimpanan Saya",
      all_files: "Semua Berkas",
      documents: "Dokumen",
      projects: "Proyek",
      sharing_section: "Kolaborasi & Berbagi",
      shared_with_me: "Dibagikan dengan Saya",
      public_links: "Tautan Berbagi Publik",
      activity_section: "Aktivitas",
      recents: "Terbaru",
      trash: "Tempat Sampah",
      connectors_section: "Integrasi Cloud",
      gdrive_sync: "Google Drive Sync",

      // Finder Toolbar & Headers
      search_placeholder: "Cari",
      col_name: "Nama",
      col_date: "Tanggal Diubah",
      col_size: "Ukuran",
      col_kind: "Jenis",
      folder_kind: "Folder",
      empty_folder: "Folder Ini Kosong",
      empty_folder_desc: "Buat dokumen WordCraft, GriidCraft, DeckCraft baru atau unggah berkas dari komputer Anda.",
      btn_back: "Kembali",
      btn_forward: "Maju",
      btn_new_folder: "Folder Baru",
      btn_new_doc: "Buat Dokumen Baru",
      btn_upload: "Unggah Berkas",
      btn_rename: "Ganti Nama",
      btn_download: "Unduh",
      btn_share: "Bagikan",
      btn_delete: "Hapus",

      // Dialogs & Modals
      create_new_file: "Buat Berkas / Dokumen Baru",
      select_doc_type: "Pilih Jenis Dokumen:",
      file_name_label: "Nama Berkas:",
      btn_cancel: "Batal",
      btn_create_open: "Buat & Buka Langsung",
      share_modal_title: "Bagikan Berkas",
      tab_public_share: "Tautan Publik (External)",
      tab_internal_share: "Berbagi Internal (User)",
      public_share_desc: "Siapapun dengan tautan ini dapat melihat berkas tanpa harus memiliki akun BijiOS.",
      password_label: "Kata Sandi (Opsional):",
      access_rights: "Hak Akses:",
      read_only: "Hanya Melihat (Read-only)",
      edit_collab: "Melihat & Edit (Collaborative)",
      btn_generate_link: "Buat Tautan Publik",
      btn_copy_link: "Salin Tautan ke Clipboard",
      internal_share_desc: "Pilih rekan tim dalam workspace tenant untuk diberikan akses langsung.",
      select_team_user: "Pilih Pengguna Tim:",
      btn_save_access: "Simpan Hak Akses",
      rename_title: "Ganti Nama Berkas / Folder",
      rename_label: "Nama Baru",
      rename_save: "Simpan",
      rename_err: "Gagal mengubah nama: ",
      delete_confirm: "Hapus berkas ini secara permanen?",

      // Tahoe Lock Screen
      wifi_connected: "WiFi Terhubung",
      login_card_tenant: "Enterprise Cloud Workspace",
      password_placeholder: "Kata Sandi",
      choose_demo_user: "Pilih Akun:",
      sleep_btn: "Tidur",
      restart_btn: "Mulai Ulang",
      shutdown_btn: "Matikan",
      login_err_username: "Silakan masukkan username atau email Anda.",
      login_err_password: "Silakan masukkan kata sandi Anda.",
      login_err_invalid: "Username atau kata sandi tidak valid.",
      login_err_network: "Gagal menghubungi server. Periksa koneksi jaringan.",
      restart_confirm: "Restart sistem BijiOS sekarang?",

      // Apps
      app_finder: "Finder (File Manager)",
      app_wordcraft: "WordCraft",
      app_griidcraft: "GriidCraft (Spreadsheet)",
      app_deckcraft: "DeckCraft (Presentasi)",
      app_pdf: "PDF Viewer",
      app_notes: "Notes (Catatan)",
      app_meet: "BijiMeet (WebRTC)",
      app_gdrive: "Konektor Google Drive",
      app_settings: "Pengaturan & API Docs",
      app_wallpaper: "Ubah Wallpaper",
      app_preview: "Pratinjau Gambar"
    },
    en: {
      // Menu Bar & Apple Menu
      app_name: "Finder",
      file_menu: "File",
      edit_menu: "Edit",
      view_menu: "View",
      go_menu: "Go",
      window_menu: "Window",
      help_menu: "Help",
      about_dokuhub: "About This BijiOS",
      activity_monitor: "Activity Monitor...",
      sys_settings: "System Settings...",
      change_wallpaper: "Change Desktop Wallpaper...",
      lock_screen: "Lock Screen",
      sleep_mode: "Sleep",
      restart_system: "Restart...",
      logout_session: "Log Out",

      // Topbar User Menu
      user_profile: "👤 Profile & Password...",
      user_appearance: "🎨 Appearance & Dock...",
      user_rbac: "👥 Users & RBAC Settings...",
      user_lock: "🔒 Lock Screen",
      user_logout: "🚪 Log Out",

      // Finder Sidebar
      storage_section: "My Storage",
      all_files: "All Files",
      documents: "Documents",
      projects: "Projects",
      sharing_section: "Sharing & Collab",
      shared_with_me: "Shared with Me",
      public_links: "Public Shared Links",
      activity_section: "Activity",
      recents: "Recents",
      trash: "Trash",
      connectors_section: "Cloud Connectors",
      gdrive_sync: "Google Drive Sync",

      // Finder Toolbar & Headers
      search_placeholder: "Search",
      col_name: "Name",
      col_date: "Date Modified",
      col_size: "Size",
      col_kind: "Kind",
      folder_kind: "Folder",
      empty_folder: "This Folder is Empty",
      empty_folder_desc: "Create a new WordCraft, GriidCraft, DeckCraft document or upload files from your computer.",
      btn_back: "Back",
      btn_forward: "Forward",
      btn_new_folder: "New Folder",
      btn_new_doc: "New Document",
      btn_upload: "Upload File",
      btn_rename: "Rename",
      btn_download: "Download",
      btn_share: "Share",
      btn_delete: "Delete",

      // Dialogs & Modals
      create_new_file: "Create New Document / File",
      select_doc_type: "Select Document Type:",
      file_name_label: "File Name:",
      btn_cancel: "Cancel",
      btn_create_open: "Create & Open Now",
      share_modal_title: "Share File",
      tab_public_share: "Public Link (External)",
      tab_internal_share: "Internal Sharing (Team)",
      public_share_desc: "Anyone with this public link can view this file without requiring a BijiOS account.",
      password_label: "Password (Optional):",
      access_rights: "Permissions:",
      read_only: "Can View (Read-only)",
      edit_collab: "Can Edit (Collaborative)",
      btn_generate_link: "Generate Public Link",
      btn_copy_link: "Copy Link to Clipboard",
      internal_share_desc: "Select a team member inside this workspace tenant to grant direct access.",
      select_team_user: "Select Team Member:",
      btn_save_access: "Save Access Permissions",
      rename_title: "Rename File / Folder",
      rename_label: "New Name",
      rename_save: "Save",
      rename_err: "Failed to rename: ",
      delete_confirm: "Permanently delete this file from BijiOS?",

      // Tahoe Lock Screen
      wifi_connected: "WiFi Connected",
      login_card_tenant: "Enterprise Cloud Workspace",
      password_placeholder: "Enter Password",
      choose_demo_user: "Select Account:",
      sleep_btn: "Sleep",
      restart_btn: "Restart",
      shutdown_btn: "Shut Down",
      login_err_username: "Please enter your username or email.",
      login_err_password: "Please enter your password.",
      login_err_invalid: "Invalid username or password.",
      login_err_network: "Unable to connect to server. Check network connection.",
      restart_confirm: "Restart BijiOS system now?",

      // Apps
      app_finder: "Finder (File Manager)",
      app_wordcraft: "WordCraft",
      app_griidcraft: "GriidCraft (Spreadsheet)",
      app_deckcraft: "DeckCraft (Presentation)",
      app_pdf: "PDF Viewer",
      app_notes: "Notes",
      app_meet: "BijiMeet (WebRTC)",
      app_gdrive: "Google Drive Connector",
      app_settings: "Settings & API Docs",
      app_wallpaper: "Change Wallpaper",
      app_preview: "Image Preview"
    }
  },

  t(key) {
    const dict = this.translations[this.lang] || this.translations.en;
    return dict[key] || this.translations.en[key] || key;
  },

  setLanguage(l) {
    this.lang = (l === 'id' ? 'id' : 'en');
    localStorage.setItem('dokuhub_lang', this.lang);
    document.documentElement.setAttribute('lang', this.lang);
    this.applyAll();
  },

  toggleLanguage() {
    this.setLanguage(this.lang === 'id' ? 'en' : 'id');
  },

  applyAll() {
    // 1. Language pill in menubar
    const langBtn = document.getElementById('menubar-lang-toggle');
    if (langBtn) {
      langBtn.innerHTML = this.lang === 'id' ? '🇮🇩 ID' : '🇬🇧 EN';
    }

    // 2. Menubar top-level items
    const menuFile = document.getElementById('menu-file');
    if (menuFile) menuFile.textContent = this.t('file_menu');
    const menuSearch = document.getElementById('menu-search');
    if (menuSearch) menuSearch.textContent = `${this.t('search_placeholder')} (⌘K)`;

    // 3. Apple Menu Dropdown
    const appleAbout = document.getElementById('apple-menu-about');
    if (appleAbout) appleAbout.textContent = this.t('about_dokuhub');
    const appleActivity = document.getElementById('apple-menu-activity');
    if (appleActivity) appleActivity.textContent = this.t('activity_monitor');
    const appleSettings = document.getElementById('apple-menu-settings');
    if (appleSettings) appleSettings.textContent = this.t('sys_settings');
    const appleWallpaper = document.getElementById('apple-menu-wallpaper');
    if (appleWallpaper) appleWallpaper.textContent = this.t('change_wallpaper');
    const appleLock = document.getElementById('apple-menu-lock');
    if (appleLock) appleLock.textContent = `🔒 ${this.t('lock_screen')}`;
    const appleSleep = document.getElementById('apple-menu-sleep');
    if (appleSleep) appleSleep.textContent = `💤 ${this.t('sleep_mode')}`;
    const appleRestart = document.getElementById('apple-menu-restart');
    if (appleRestart) appleRestart.textContent = `🔄 ${this.t('restart_system')}`;
    const appleLogout = document.getElementById('apple-menu-logout');
    if (appleLogout) appleLogout.textContent = `🚪 ${this.t('logout_session')}`;

    // 4. User Dropdown Menu
    const userProfile = document.getElementById('user-menu-profile');
    if (userProfile) userProfile.textContent = this.t('user_profile');
    const userAppearance = document.getElementById('user-menu-appearance');
    if (userAppearance) userAppearance.textContent = this.t('user_appearance');
    const userRbac = document.getElementById('dropdown-menu-rbac-link');
    if (userRbac) userRbac.textContent = this.t('user_rbac');
    const userLock = document.getElementById('user-menu-lock');
    if (userLock) userLock.textContent = this.t('user_lock');
    const userLogout = document.getElementById('user-menu-logout');
    if (userLogout) userLogout.textContent = this.t('user_logout');

    // 5. Tahoe Login items
    const loginTitle = document.getElementById('tahoe-login-title');
    if (loginTitle) loginTitle.textContent = this.lang === 'id' ? 'Masuk ke BijiOS' : 'Sign In to BijiOS';
    const wifiEl = document.querySelector('.tahoe-glass-pill span:first-child');
    if (wifiEl) wifiEl.textContent = `📶 ${this.t('wifi_connected')}`;
    const passInput = document.getElementById('tahoe-pass-input');
    if (passInput) passInput.placeholder = this.t('password_placeholder');

    const actions = document.querySelectorAll('.tahoe-action-btn span');
    if (actions.length >= 3) {
      actions[0].textContent = this.t('sleep_btn');
      actions[1].textContent = this.t('restart_btn');
      actions[2].textContent = this.t('shutdown_btn');
    }

    // 6. Dock tooltips
    const dockFinder = document.querySelector('.dock-item[data-app="finder"] .dock-tooltip');
    if (dockFinder) dockFinder.textContent = this.t('app_finder');
    const dockWord = document.querySelector('.dock-item[data-app="wordcraft"] .dock-tooltip');
    if (dockWord) dockWord.textContent = this.t('app_wordcraft');
    const dockGriid = document.querySelector('.dock-item[data-app="griidcraft"] .dock-tooltip');
    if (dockGriid) dockGriid.textContent = this.t('app_griidcraft');
    const dockDeck = document.querySelector('.dock-item[data-app="deckcraft"] .dock-tooltip');
    if (dockDeck) dockDeck.textContent = this.t('app_deckcraft');
    const dockNotes = document.querySelector('.dock-item[data-app="notes"] .dock-tooltip');
    if (dockNotes) dockNotes.textContent = this.t('app_notes');
    const dockMeet = document.querySelector('.dock-item[data-app="meet"] .dock-tooltip');
    if (dockMeet) dockMeet.textContent = this.t('app_meet');
    const dockSettings = document.querySelector('.dock-item[data-app="settings"] .dock-tooltip');
    if (dockSettings) dockSettings.textContent = this.t('app_settings');
    const dockWallpaper = document.querySelector('.dock-item[data-app="wallpaper"] .dock-tooltip');
    if (dockWallpaper) dockWallpaper.textContent = this.t('app_wallpaper');

    // 7. Refresh open apps
    if (window.DokuFinder && DokuFinder.container) {
      DokuFinder.renderLayout();
      DokuFinder.renderCurrentView();
    }
    if (window.DokuSettings && DokuSettings.container) {
      DokuSettings.render();
    }
    if (window.DokuWordCraft && DokuWordCraft.container && typeof DokuWordCraft.renderRibbon === 'function') {
      DokuWordCraft.renderRibbon();
    }
    if (window.DokuGriidCraft && DokuGriidCraft.container && typeof DokuGriidCraft.render === 'function') {
      DokuGriidCraft.render();
      if (typeof DokuGriidCraft.updateWindowTitle === 'function') {
        DokuGriidCraft.updateWindowTitle();
      }
    }
    if (window.DokuWidgets && typeof DokuWidgets.render === 'function') {
      DokuWidgets.render();
    }
    if (window.DokuNotes && DokuNotes.container && typeof DokuNotes.renderSidebar === 'function') {
      DokuNotes.renderSidebar();
    }
    if (window.DokuAbout && DokuAbout.containerEl) {
      DokuAbout.render();
      const aboutWinTitle = document.querySelector('#window-about .window-title');
      if (aboutWinTitle) {
        aboutWinTitle.textContent = this.lang === 'id' ? '🍏 Tentang BijiOS' : '🍏 About This BijiOS';
      }
    }
    if (window.DokuActivity && DokuActivity.containerEl) {
      DokuActivity.render();
      const actWinTitle = document.querySelector('#window-activity .window-title');
      if (actWinTitle) {
        actWinTitle.textContent = this.lang === 'id' ? '📈 Monitor Aktivitas' : '📈 Activity Monitor';
      }
    }
    if (window.DokuPreview && DokuPreview.containerEl) {
      DokuPreview.render();
      DokuPreview.updateWindowTitle();
    }
    if (window.DokuPdfViewer && DokuPdfViewer.container) {
      DokuPdfViewer.render();
      DokuPdfViewer.updateWindowTitle();
    }
    if (window.DokuWallpaper && DokuWallpaper.container) {
      DokuWallpaper.render();
    }
    if (window.DokuGDrive && DokuGDrive.container && typeof DokuGDrive.render === 'function') {
      DokuGDrive.render();
    }
    if (window.DokuMeet && DokuMeet.container && typeof DokuMeet.render === 'function') {
      DokuMeet.render();
    }
    if (window.DokuGDrive && DokuGDrive.container && typeof DokuGDrive.render === 'function') {
      DokuGDrive.render();
    }
    if (window.DokuMeet && DokuMeet.container && !DokuMeet.roomCode && typeof DokuMeet.renderLobby === 'function') {
      DokuMeet.renderLobby();
    }
    if (window.DokuDeckCraft && DokuDeckCraft.container && typeof DokuDeckCraft.render === 'function') {
      DokuDeckCraft.render();
      if (typeof DokuDeckCraft.updateWindowTitle === 'function') {
        DokuDeckCraft.updateWindowTitle();
      }
    }
    if (window.DokuCalendar && DokuCalendar.container && typeof DokuCalendar.render === 'function') {
      DokuCalendar.render();
    }
    if (window.DokuCalculator && DokuCalculator.container && typeof DokuCalculator.render === 'function') {
      DokuCalculator.render();
    }
    if (window.DokuTahoeLogin) {
      DokuTahoeLogin.initClock();
    }
    if (window.DokuHub) {
      DokuHub.initClock();
    }
  }
};
