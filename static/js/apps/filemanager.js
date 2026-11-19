/**
 * DokuHub Finder - Native macOS Dark Mode Tree & File Manager
 * Faithful reproduction of macOS Ventura/Sonoma/Tahoe Finder:
 * - Tree/List hierarchical view with expandable chevrons (› / ⌄)
 * - Icons Grid, Miller Columns, and Gallery views
 * - Sidebar: Recents, Shared, Favorites, Locations, and Colored Tags
 * - Real File/Folder Creation, Real File Upload, and Sharing System
 * - Dual Language (EN & ID) support with DokuI18n (defaults to English)
 * - Clean Real Storage: No mockups / dummy data
 */

window.DokuFinder = {
  container: null,
  currentFolderId: 'root',
  currentCategory: 'all',
  currentView: 'tree', // 'tree' (list/tree), 'icons', 'columns', 'gallery'
  selectedFileId: null,
  allFiles: [],
  expandedFolders: new Set(['fld_docs', 'fld_projects']),
  folderHistory: ['root'],
  historyIndex: 0,
  searchQuery: '',

  async init(containerEl) {
    this.container = containerEl;
    this.currentFolderId = 'root';
    this.folderHistory = ['root'];
    this.historyIndex = 0;
    this.searchQuery = '';
    this.expandedFolders = new Set(['fld_docs', 'fld_projects']);
    if (!this._hasAttachedKeyEvents) {
      this._hasAttachedKeyEvents = true;
      window.addEventListener('keydown', (e) => {
        // Never hijack keys if user is typing in editor (WordCraft contenteditable), input, or textarea
        if (e.target && (e.target.isContentEditable || e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA')) {
          return;
        }
        if (document.activeElement && (document.activeElement.isContentEditable || document.activeElement.tagName === 'INPUT' || document.activeElement.tagName === 'TEXTAREA')) {
          return;
        }

        // Only handle if Finder is the active frontmost window
        if (window.DokuHub && DokuHub.activeAppId && DokuHub.activeAppId !== 'finder') {
          return;
        }

        if (e.key === 'F2') {
          if (this.selectedFileId) {
            e.preventDefault();
            this.openRenameModal();
          }
        } else if (e.key === 'Delete' || (e.key === 'Backspace' && (e.metaKey || e.ctrlKey))) {
          if (this.selectedFileId) {
            e.preventDefault();
            this.deleteSelected();
          }
        }
      });
    }
    await this.renderLayout();
    await this.loadAllFiles();
  },

  /* ------------------------------------------------------------------------
     Layout Rendering
     ------------------------------------------------------------------------ */
  async renderLayout() {
    if (!this.container) return;
    const t = (k) => window.DokuI18n ? window.DokuI18n.t(k) : k;
    const userName = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.name : 'dionipe';

    this.container.innerHTML = `
      <div class="finder-layout">
        <!-- DokuHub Sovereign Workspace Sidebar -->
        <div class="finder-sidebar">
          <!-- 1. My Storage / Workspace -->
          <div style="margin-bottom:14px;">
            <div class="sidebar-section-title">${DokuI18n.lang === 'id' ? 'Penyimpanan Saya' : 'My Storage'}</div>
            <div class="sidebar-item ${this.currentCategory === 'all' && this.currentFolderId === 'root' ? 'active' : ''}" id="sidebar-item-all" onclick="DokuFinder.navigateToFolder('root', '${userName}')">
              <span style="font-size:14px; margin-right:6px;">🏠</span> ${DokuI18n.lang === 'id' ? 'Semua Berkas' : 'All Files'}
            </div>
            <div class="sidebar-item ${this.currentFolderId === 'fld_docs' ? 'active' : ''}" id="sidebar-item-docs" onclick="DokuFinder.navigateToDocuments()">
              <span style="font-size:14px; margin-right:6px;">📄</span> ${DokuI18n.lang === 'id' ? 'Dokumen' : 'Documents'}
            </div>
            <div class="sidebar-item ${this.currentFolderId === 'fld_projects' ? 'active' : ''}" id="sidebar-item-projects" onclick="DokuFinder.navigateToProjects()">
              <span style="font-size:14px; margin-right:6px;">📁</span> ${DokuI18n.lang === 'id' ? 'Proyek' : 'Projects'}
            </div>
          </div>

          <!-- 2. Sharing & Collaboration -->
          <div style="margin-bottom:14px;">
            <div class="sidebar-section-title">${DokuI18n.lang === 'id' ? 'Kolaborasi & Berbagi' : 'Sharing & Collab'}</div>
            <div class="sidebar-item ${this.currentCategory === 'shared' ? 'active' : ''}" onclick="DokuFinder.setCategory('shared')">
              <span style="font-size:14px; margin-right:6px;">👥</span> ${DokuI18n.lang === 'id' ? 'Dibagikan dengan Saya' : 'Shared with Me'}
            </div>
            <div class="sidebar-item ${this.currentCategory === 'public' ? 'active' : ''}" onclick="DokuFinder.setCategory('public')">
              <span style="font-size:14px; margin-right:6px;">🌐</span> ${DokuI18n.lang === 'id' ? 'Tautan Berbagi Publik' : 'Public Shared Links'}
            </div>
          </div>

          <!-- 3. Activity & Maintenance -->
          <div style="margin-bottom:14px;">
            <div class="sidebar-section-title">${DokuI18n.lang === 'id' ? 'Aktivitas' : 'Activity'}</div>
            <div class="sidebar-item ${this.currentCategory === 'recents' ? 'active' : ''}" onclick="DokuFinder.setCategory('recents')">
              <span style="font-size:14px; margin-right:6px;">🕒</span> ${DokuI18n.lang === 'id' ? 'Terbaru' : 'Recents'}
            </div>
            <div class="sidebar-item ${this.currentCategory === 'trash' ? 'active' : ''}" onclick="DokuFinder.setCategory('trash')">
              <span style="font-size:14px; margin-right:6px;">🗑️</span> ${DokuI18n.lang === 'id' ? 'Tempat Sampah' : 'Trash'}
            </div>
          </div>

          <!-- 4. Integrations -->
          <div>
            <div class="sidebar-section-title">${DokuI18n.lang === 'id' ? 'Integrasi Cloud' : 'Cloud Connectors'}</div>
            <div class="sidebar-item" onclick="DokuHub.openApp('gdrive')">
              <span style="font-size:14px; margin-right:6px;">☁️</span> Google Drive Sync
            </div>
          </div>
        </div>

        <!-- Main Work Area -->
        <div class="finder-main">
          <!-- Native macOS Header / Toolbar -->
          <div class="finder-toolbar" style="padding:6px 12px; gap:10px;">
            <!-- Left: Back / Forward Nav -->
            <div class="finder-nav-arrows">
              <button class="finder-nav-btn" id="btn-finder-back" onclick="DokuFinder.navBack()" title="${t('btn_back')}">‹</button>
              <button class="finder-nav-btn" id="btn-finder-forward" onclick="DokuFinder.navForward()" title="${t('btn_forward')}">›</button>
            </div>

            <!-- Real macOS Clickable Breadcrumbs (User -> Root -> Folder -> Files) -->
            <div class="finder-breadcrumbs" id="finder-breadcrumbs" style="display:flex; align-items:center; gap:6px; font-weight:600; font-size:12px; color:var(--text-main); min-width:140px;">
              <span class="crumb active" onclick="DokuFinder.navigateToFolder('root', '${userName}')" style="cursor:pointer; font-weight:700;">🏠 ${userName}</span>
            </div>

            <!-- Segmented View Switcher: Icons (⊞), List/Tree (☰), Columns (|||), Gallery (🔲) -->
            <div class="finder-segmented-view">
              <button class="finder-segment-btn ${this.currentView === 'icons' ? 'active' : ''}" onclick="DokuFinder.setView('icons')" title="Icons View">⊞</button>
              <button class="finder-segment-btn ${this.currentView === 'tree' ? 'active' : ''}" onclick="DokuFinder.setView('tree')" title="List / Tree View">☰</button>
              <button class="finder-segment-btn ${this.currentView === 'columns' ? 'active' : ''}" onclick="DokuFinder.setView('columns')" title="Columns View">|||</button>
              <button class="finder-segment-btn ${this.currentView === 'gallery' ? 'active' : ''}" onclick="DokuFinder.setView('gallery')" title="Gallery View">🔲</button>
            </div>

            <!-- Action Buttons Group -->
            <div class="finder-toolbar-actions">
              <button class="finder-icon-btn" onclick="DokuFinder.toggleSortMenu()" title="Arrangement">⚏</button>
              <button class="finder-icon-btn" id="btn-quick-share" onclick="DokuFinder.openShareModal()" title="${t('btn_share')}">↗</button>
              <button class="finder-icon-btn" onclick="DokuFinder.openTagPicker()" title="Tag">🏷️</button>
              <button class="finder-icon-btn" onclick="DokuFinder.toggleActionMenu()" title="Actions">⋯</button>
              
              <!-- Quick Action Pills -->
              <button class="mac-btn" onclick="DokuFinder.openNewDialog('folder')" style="padding:2px 8px; font-size:11px;">
                📁 + ${t('btn_new_folder')}
              </button>
              <button class="mac-btn primary" onclick="DokuFinder.openNewFileDialog()" style="padding:2px 8px; font-size:11px;">
                ✨ + ${t('btn_new_doc')}
              </button>
              <label class="mac-btn" style="cursor:pointer; padding:2px 8px; font-size:11px; margin-bottom:0;" title="${t('btn_upload')}">
                ⬆️ ${t('btn_upload')}
                <input type="file" id="finder-file-uploader" style="display:none;" onchange="DokuFinder.handleFileUpload(event)">
              </label>
              <button class="mac-btn" id="btn-quick-rename" disabled onclick="DokuFinder.openRenameModal()" style="padding:2px 8px; font-size:11px;" title="Rename File / Folder">
                ✏️ ${DokuI18n.lang === 'id' ? 'Ganti Nama' : 'Rename'}
              </button>
              <button class="mac-btn" id="btn-quick-download" disabled onclick="DokuFinder.downloadSelected()" style="padding:2px 8px; font-size:11px;" title="Download Physical File">
                ⬇️ Download
              </button>
              <button class="mac-btn" id="btn-quick-qr" disabled onclick="DokuFinder.openDownloadQrModal()" style="padding:2px 8px; font-size:11px;" title="${DokuI18n.lang === 'id' ? 'QR Code Download & Share' : 'QR Code Download & Share'}">
                📱 QR
              </button>
              <button class="mac-btn" id="btn-quick-delete" disabled onclick="DokuFinder.deleteSelected()" style="padding:2px 8px; font-size:11px;" title="${t('btn_delete')}">
                🗑️
              </button>
            </div>

            <div style="flex:1;"></div>

            <!-- Search Input -->
            <div style="position:relative; display:flex; align-items:center;">
              <span style="position:absolute; left:7px; font-size:11px; opacity:0.6; pointer-events:none;">🔍</span>
              <input type="text" class="finder-search-input" id="finder-search-input" placeholder="${t('search_placeholder')}" oninput="DokuFinder.handleSearch(this.value)">
            </div>
          </div>

          <!-- Dynamic View Container -->
          <div id="finder-view-container" style="flex:1; overflow:hidden; display:flex; flex-direction:column;">
            <!-- Tree, Icons, Columns, or Gallery injected here -->
          </div>

          <!-- Finder Status Bar (Items & Real Storage Quota) -->
          <div class="finder-status-bar" id="finder-status-bar">
            <span id="finder-status-items">Loading...</span>
            <span id="finder-status-quota" style="display:flex; align-items:center; gap:8px;">
              <span>Storage Quota</span>
            </span>
          </div>
        </div>
      </div>

      <!-- New File / Document Creation Modal -->
      <div id="new-file-modal-overlay" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(10px); z-index:999999; align-items:center; justify-content:center;">
        <div style="width:480px; max-width:92vw; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:18px; box-shadow:0 30px 80px rgba(0,0,0,0.4); padding:24px;">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
            <h3 style="font-size:17px; font-weight:700;">${t('create_new_file')}</h3>
            <button class="traffic-btn btn-close" onclick="DokuFinder.closeNewFileDialog()"></button>
          </div>

          <div style="margin-bottom:16px;">
            <label style="font-size:12px; font-weight:600; display:block; margin-bottom:8px;">${t('select_doc_type')}</label>
            <div style="display:grid; grid-template-columns:repeat(2, 1fr); gap:10px;">
              <div class="new-type-card active" data-ext="docx" onclick="DokuFinder.selectDocType(this, 'docx')" style="border:2px solid var(--accent-color); border-radius:10px; padding:12px; cursor:pointer; background:var(--card-bg); text-align:center;">
                <div style="font-size:28px; margin-bottom:4px;">📝</div>
                <div style="font-weight:700; font-size:13px;">WordCraft (.docx)</div>
                <div style="font-size:11px; color:var(--text-muted);">${DokuI18n.lang === 'id' ? 'Dokumen Word Resmi' : 'Word Processor Document'}</div>
              </div>
              <div class="new-type-card" data-ext="xlsx" onclick="DokuFinder.selectDocType(this, 'xlsx')" style="border:2px solid var(--window-border); border-radius:10px; padding:12px; cursor:pointer; background:var(--card-bg); text-align:center;">
                <div style="font-size:28px; margin-bottom:4px;">📊</div>
                <div style="font-weight:700; font-size:13px;">GriidCraft (.xlsx)</div>
                <div style="font-size:11px; color:var(--text-muted);">${DokuI18n.lang === 'id' ? 'Spreadsheet & Rumus' : 'Spreadsheet & Live Formulas'}</div>
              </div>
              <div class="new-type-card" data-ext="pptx" onclick="DokuFinder.selectDocType(this, 'pptx')" style="border:2px solid var(--window-border); border-radius:10px; padding:12px; cursor:pointer; background:var(--card-bg); text-align:center;">
                <div style="font-size:28px; margin-bottom:4px;">📽️</div>
                <div style="font-weight:700; font-size:13px;">DeckCraft (.pptx)</div>
                <div style="font-size:11px; color:var(--text-muted);">${DokuI18n.lang === 'id' ? 'Presentasi & Slide' : 'Presentation & Slides'}</div>
              </div>
              <div class="new-type-card" data-ext="md" onclick="DokuFinder.selectDocType(this, 'md')" style="border:2px solid var(--window-border); border-radius:10px; padding:12px; cursor:pointer; background:var(--card-bg); text-align:center;">
                <div style="font-size:28px; margin-bottom:4px;">Ⓜ️</div>
                <div style="font-weight:700; font-size:13px;">Markdown (.md)</div>
                <div style="font-size:11px; color:var(--text-muted);">${DokuI18n.lang === 'id' ? 'Dokumen Markdown & Auto Preview' : 'Markdown Doc & Auto Preview'}</div>
              </div>
              <div class="new-type-card" data-ext="folder" onclick="DokuFinder.selectDocType(this, 'folder')" style="border:2px solid var(--window-border); border-radius:10px; padding:12px; cursor:pointer; background:var(--card-bg); text-align:center; grid-column: span 2;">
                <div style="font-size:24px; margin-bottom:2px;">📁</div>
                <div style="font-weight:700; font-size:13px;">${t('btn_new_folder')}</div>
                <div style="font-size:11px; color:var(--text-muted);">${t('folder_kind')}</div>
              </div>
            </div>
          </div>

          <div style="margin-bottom:20px;">
            <label style="font-size:12px; font-weight:600; display:block; margin-bottom:6px;">${t('file_name_label')}</label>
            <input type="text" id="new-file-name-input" placeholder="${DokuI18n.lang === 'id' ? 'Ketik nama berkas...' : 'Enter file name...'}" value="" style="width:100%; padding:10px 14px; border:1px solid var(--window-border); border-radius:8px; font-size:14px; background:var(--card-bg); color:var(--text-main); outline:none;" onkeydown="if(event.key==='Enter') DokuFinder.submitNewFile()">
          </div>

          <div style="display:flex; justify-content:flex-end; gap:8px;">
            <button class="mac-btn" onclick="DokuFinder.closeNewFileDialog()">${t('btn_cancel')}</button>
            <button class="mac-btn primary" onclick="DokuFinder.submitNewFile()">${t('btn_create_open')}</button>
          </div>
        </div>
      </div>

      <!-- Rename File / Folder Modal -->
      <div id="rename-modal-overlay" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(10px); z-index:999999; align-items:center; justify-content:center;">
        <div style="width:440px; max-width:92vw; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:18px; box-shadow:0 30px 80px rgba(0,0,0,0.4); padding:24px;">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
            <h3 style="font-size:17px; font-weight:700;">✏️ ${DokuI18n.lang === 'id' ? 'Ganti Nama Berkas / Folder' : 'Rename File / Folder'}</h3>
            <button class="traffic-btn btn-close" onclick="DokuFinder.closeRenameModal()"></button>
          </div>

          <div style="margin-bottom:20px;">
            <label style="font-size:12px; font-weight:600; display:block; margin-bottom:6px;">${DokuI18n.lang === 'id' ? 'Nama Baru' : 'New Name'}</label>
            <input type="text" id="rename-file-input" style="width:100%; padding:10px 14px; border:1px solid var(--window-border); border-radius:8px; font-size:14px; background:var(--card-bg); color:var(--text-main); outline:none;" onkeydown="if(event.key==='Enter') DokuFinder.submitRename(); if(event.key==='Escape') DokuFinder.closeRenameModal();">
          </div>

          <div style="display:flex; justify-content:flex-end; gap:8px;">
            <button class="mac-btn" onclick="DokuFinder.closeRenameModal()">${t('btn_cancel')}</button>
            <button class="mac-btn primary" onclick="DokuFinder.submitRename()">${DokuI18n.lang === 'id' ? 'Simpan' : 'Save'}</button>
          </div>
        </div>
      </div>

      <!-- Sharing Modal Overlay -->
      <div id="share-modal-overlay" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(10px); z-index:999999; align-items:center; justify-content:center;">
        <div style="width:460px; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:16px; box-shadow:0 30px 70px rgba(0,0,0,0.35); padding:24px;">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:14px;">
            <h3 style="font-size:16px; font-weight:700;" id="share-modal-title">${t('share_modal_title')}</h3>
            <button class="traffic-btn btn-close" onclick="DokuFinder.closeShareModal()"></button>
          </div>

          <!-- Share Tabs -->
          <div style="display:flex; gap:4px; border-bottom:1px solid var(--window-border); margin-bottom:14px; padding-bottom:6px;">
            <button class="mac-btn primary" id="tab-btn-pub" onclick="DokuFinder.switchShareTab('public')" style="font-size:11.5px; padding:5px 10px;">🌐 ${DokuI18n.lang === 'id' ? 'Publik' : 'Public'}</button>
            <button class="mac-btn" id="tab-btn-int" onclick="DokuFinder.switchShareTab('internal')" style="font-size:11.5px; padding:5px 10px;">🏢 ${DokuI18n.lang === 'id' ? 'Internal Staff' : 'Internal Staff'}</button>
            <button class="mac-btn" id="tab-btn-ext" onclick="DokuFinder.switchShareTab('external')" style="font-size:11.5px; padding:5px 10px;">✉️ ${DokuI18n.lang === 'id' ? 'External' : 'External'}</button>
          </div>

          <!-- Public Share Tab (NO Approval Flow Needed) -->
          <div id="share-tab-public">
            <p style="font-size:12px; color:var(--text-muted); margin-bottom:10px;">
              ${DokuI18n.lang === 'id' ? 'Tautan publik aktif langsung tanpa memerlukan persetujuan (approval) Supervisor.' : 'Public link is immediately active with no Supervisor approval flow required.'}
            </p>
            <div style="margin-bottom:10px;">
              <label style="font-size:11px; font-weight:600;">${t('password_label')}</label>
              <input type="password" id="share-public-pw" placeholder="${DokuI18n.lang === 'id' ? 'Biarkan kosong jika tanpa kata sandi' : 'Leave empty for public open access'}" style="width:100%; padding:6px 8px; border:1px solid var(--window-border); border-radius:6px; font-size:12px; margin-top:4px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div style="margin-bottom:14px;">
              <label style="font-size:11px; font-weight:600;">${t('access_rights')}</label>
              <select id="share-public-perm" style="width:100%; padding:6px 8px; border:1px solid var(--window-border); border-radius:6px; font-size:12px; margin-top:4px; background:var(--card-bg); color:var(--text-main);">
                <option value="view">👁️ ${DokuI18n.lang === 'id' ? 'View Only (Hanya Lihat)' : 'View Only'}</option>
                <option value="comment">💬 ${DokuI18n.lang === 'id' ? 'Comment (Lihat & Beri Catatan)' : 'Comment (Add Notes)'}</option>
                <option value="edit">✏️ ${DokuI18n.lang === 'id' ? 'Editor (Lihat & Sunting)' : 'Editor (Full Edit)'}</option>
              </select>
            </div>
            <button class="mac-btn primary" style="width:100%; justify-content:center; padding:8px;" onclick="DokuFinder.generatePublicShare()">${t('btn_generate_link')}</button>
            <div id="share-public-result" style="display:none; margin-top:14px; padding:12px; background:rgba(0,122,255,0.08); border-radius:10px; border:1px solid rgba(0,122,255,0.28);">
              <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:6px;">
                <span style="font-size:11px; font-weight:700; color:var(--accent-color);">${DokuI18n.lang === 'id' ? 'Tautan & QR Code Siap:' : 'Share Link & QR Code Ready:'}</span>
                <div style="display:flex; gap:4px;">
                  <button class="mac-btn primary" id="share-qr-tab-link" style="font-size:10px; padding:2px 7px;" onclick="DokuFinder.renderShareQr('share')">🔗 QR Share</button>
                  <button class="mac-btn" id="share-qr-tab-dl" style="font-size:10px; padding:2px 7px;" onclick="DokuFinder.renderShareQr('download')">⬇️ QR Download</button>
                </div>
              </div>
              <div style="display:flex; justify-content:center; margin:8px 0;">
                <div style="padding:8px; background:#ffffff; border-radius:10px; border:1px solid rgba(0,0,0,0.08); box-shadow:0 4px 14px rgba(0,0,0,0.08);">
                  <canvas id="share-qr-canvas" width="150" height="150"></canvas>
                </div>
              </div>
              <input type="text" id="share-link-input" readonly style="width:100%; font-size:11px; padding:5px 7px; border:1px solid var(--window-border); border-radius:6px; margin-bottom:8px; background:var(--card-bg); color:var(--text-main);">
              <div style="display:flex; gap:6px;">
                <button class="mac-btn primary" style="font-size:11px; flex:1; justify-content:center;" onclick="DokuFinder.copyShareLink()">📋 ${t('btn_copy_link')}</button>
                <button class="mac-btn" style="font-size:11px; flex:1; justify-content:center;" onclick="DokuFinder.downloadShareQrPng()">⬇️ Unduh QR (PNG)</button>
              </div>
            </div>
          </div>

          <!-- Internal Staff Share Tab (Requires Supervisor Approval for Staff) -->
          <div id="share-tab-internal" style="display:none;">
            <div style="padding:8px 10px; border-radius:8px; background:rgba(245,158,11,0.12); border:1px solid rgba(245,158,11,0.3); font-size:11.5px; color:var(--text-main); margin-bottom:12px;">
              🛡️ ${DokuI18n.lang === 'id' ? 'Pengajuan oleh Staff memerlukan persetujuan (Approval) dari Supervisor Bagian sebelum akses aktif.' : 'Share requests by Staff require Supervisor approval before access becomes active.'}
            </div>
            <div style="margin-bottom:10px;">
              <label style="font-size:11px; font-weight:600;">${t('select_team_user')}</label>
              <select id="share-internal-user" style="width:100%; padding:6px 8px; border:1px solid var(--window-border); border-radius:6px; font-size:12px; margin-top:4px; background:var(--card-bg); color:var(--text-main);">
                <option value="">Select team member...</option>
              </select>
            </div>
            <div style="margin-bottom:10px;">
              <label style="font-size:11px; font-weight:600;">${t('access_rights')}</label>
              <select id="share-internal-perm" style="width:100%; padding:6px 8px; border:1px solid var(--window-border); border-radius:6px; font-size:12px; margin-top:4px; background:var(--card-bg); color:var(--text-main);">
                <option value="view">👁️ ${DokuI18n.lang === 'id' ? 'View Only (Hanya Lihat)' : 'View Only'}</option>
                <option value="comment">💬 ${DokuI18n.lang === 'id' ? 'Comment (Cukup Memberikan Catatan)' : 'Comment (Notes Only)'}</option>
                <option value="edit">✏️ ${DokuI18n.lang === 'id' ? 'Editor (Dapat Menyunting)' : 'Editor (Can Edit)'}</option>
              </select>
            </div>
            <div style="margin-bottom:14px;">
              <label style="font-size:11px; font-weight:600;">${DokuI18n.lang === 'id' ? 'Alasan / Catatan Pengajuan ke Supervisor:' : 'Request Reason for Supervisor:'}</label>
              <input type="text" id="share-internal-note" placeholder="${DokuI18n.lang === 'id' ? 'Misal: Untuk review laporan kuartal Bagian B' : 'e.g. For Q4 report review'}" style="width:100%; padding:6px 8px; border:1px solid var(--window-border); border-radius:6px; font-size:12px; margin-top:4px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <button class="mac-btn primary" style="width:100%; justify-content:center; padding:8px;" onclick="DokuFinder.saveInternalShare()">📤 ${DokuI18n.lang === 'id' ? 'Ajukan / Bagikan Akses Internal' : 'Submit / Share Internal Access'}</button>
          </div>

          <!-- External Share Tab (Requires Supervisor Approval for Staff) -->
          <div id="share-tab-external" style="display:none;">
            <div style="padding:8px 10px; border-radius:8px; background:rgba(239,68,68,0.1); border:1px solid rgba(239,68,68,0.28); font-size:11.5px; color:var(--text-main); margin-bottom:12px;">
              🔐 ${DokuI18n.lang === 'id' ? 'Berbagi ke pihak Eksternal dijaga ketat: pengajuan Staff menunggu Approval Supervisor.' : 'External sharing is strictly governed: Staff requests require Supervisor approval.'}
            </div>
            <div style="margin-bottom:10px;">
              <label style="font-size:11px; font-weight:600;">${DokuI18n.lang === 'id' ? 'Email Penerima Eksternal:' : 'External Recipient Email:'}</label>
              <input type="email" id="share-external-email" placeholder="partner@company.com" style="width:100%; padding:6px 8px; border:1px solid var(--window-border); border-radius:6px; font-size:12px; margin-top:4px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div style="margin-bottom:10px;">
              <label style="font-size:11px; font-weight:600;">${t('access_rights')}</label>
              <select id="share-external-perm" style="width:100%; padding:6px 8px; border:1px solid var(--window-border); border-radius:6px; font-size:12px; margin-top:4px; background:var(--card-bg); color:var(--text-main);">
                <option value="view">👁️ ${DokuI18n.lang === 'id' ? 'View Only (Hanya Lihat)' : 'View Only'}</option>
                <option value="comment">💬 ${DokuI18n.lang === 'id' ? 'Comment (Cukup Memberikan Catatan)' : 'Comment (Notes Only)'}</option>
                <option value="edit">✏️ ${DokuI18n.lang === 'id' ? 'Editor (Dapat Menyunting)' : 'Editor (Can Edit)'}</option>
              </select>
            </div>
            <div style="margin-bottom:14px;">
              <label style="font-size:11px; font-weight:600;">${DokuI18n.lang === 'id' ? 'Alasan / Tujuan Berbagi Eksternal:' : 'Purpose of External Sharing:'}</label>
              <input type="text" id="share-external-note" placeholder="${DokuI18n.lang === 'id' ? 'Misal: Audit eksternal oleh mitra' : 'e.g. External partner audit'}" style="width:100%; padding:6px 8px; border:1px solid var(--window-border); border-radius:6px; font-size:12px; margin-top:4px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <button class="mac-btn primary" style="width:100%; justify-content:center; padding:8px;" onclick="DokuFinder.saveExternalShare()">📤 ${DokuI18n.lang === 'id' ? 'Ajukan Berbagi ke Eksternal' : 'Submit External Share Request'}</button>
          </div>
        </div>
      </div>
    `;

    this.updateNavButtons();
  },

  /* ------------------------------------------------------------------------
     Data Loading & Hierarchy
     ------------------------------------------------------------------------ */
  async loadAllFiles() {
    try {
      const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : 'usr_dionipe_849';
      const currentTenantId = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : (window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.tenant_id : '');
      const [fRes, qRes] = await Promise.all([
        fetch(`/api/files?parent_id=all&user_id=${currentUserId}&tenant_id=${currentTenantId}&category=${this.currentCategory}`),
        fetch(`/api/storage/quota?user_id=${currentUserId}&tenant_id=${currentTenantId}`)
      ]);
      this.allFiles = await fRes.json();
      try {
        this.storageQuota = await qRes.json();
      } catch {
        this.storageQuota = null;
      }
      this.updateBreadcrumbs();
      if (Array.isArray(this.allFiles)) {
        this.allFiles.forEach(f => {
          if ((f.is_folder || f.extension === 'folder') && f.parent_id === 'root') {
            this.expandedFolders.add(f.id);
          }
        });
      }
      this.renderCurrentView();
      this.updateStatusBar();

      if (!Array.isArray(this.allFiles) || this.allFiles.length === 0) {
        setTimeout(async () => {
          try {
            const retryRes = await fetch(`/api/files?parent_id=all&user_id=${currentUserId}&tenant_id=${currentTenantId}&category=all`);
            const retryFiles = await retryRes.json();
            if (Array.isArray(retryFiles) && retryFiles.length > 0) {
              this.allFiles = retryFiles;
              this.renderCurrentView();
              this.updateStatusBar();
            }
          } catch (_) {}
        }, 300);
      }
    } catch (e) {
      console.error("Failed to load files:", e);
    }
  },

  updateStatusBar() {
    const itemsEl = document.getElementById('finder-status-items');
    const quotaEl = document.getElementById('finder-status-quota');
    if (itemsEl) {
      const count = this.allFiles.length;
      itemsEl.textContent = `${count} ${count === 1 ? 'item' : 'items'}`;
    }
    if (quotaEl && this.storageQuota) {
      const q = this.storageQuota;
      quotaEl.innerHTML = `
        <span>Storage: <strong>${q.used_mb} MB</strong> of <strong>${q.quota_mb} MB</strong> (${q.percent}%)</span>
        <div style="width:70px; height:5px; background:rgba(125,125,125,0.25); border-radius:3px; overflow:hidden;">
          <div style="width:${Math.min(100, q.percent)}%; height:100%; background:${q.percent > 85 ? '#ef4444' : '#10b981'};"></div>
        </div>
      `;
    }
  },

  render() {
    this.renderLayout();
    this.renderCurrentView();
  },

  renderCurrentView() {
    const container = document.getElementById('finder-view-container');
    if (!container) return;

    if (this.currentView === 'tree') {
      this.renderTreeView(container);
    } else if (this.currentView === 'icons') {
      this.renderIconsView(container);
    } else if (this.currentView === 'columns') {
      this.renderColumnsView(container);
    } else if (this.currentView === 'gallery') {
      this.renderGalleryView(container);
    }

    this.updateToolbarSelection();
  },

  /* ------------------------------------------------------------------------
     1. Tree / List View
     ------------------------------------------------------------------------ */
  renderTreeView(container) {
    const t = (k) => window.DokuI18n ? window.DokuI18n.t(k) : k;

    let rootItems = [];
    if (this.searchQuery) {
      const q = this.searchQuery.toLowerCase();
      rootItems = this.allFiles.filter(f => f.name.toLowerCase().includes(q));
    } else if (this.currentCategory === 'shared') {
      rootItems = this.allFiles.filter(f => f.is_shared_internal);
    } else if (this.currentCategory === 'public') {
      rootItems = this.allFiles.filter(f => f.is_shared_public);
    } else if (this.currentCategory === 'recents') {
      rootItems = [...this.allFiles].sort((a, b) => b.updated_at - a.updated_at).slice(0, 15);
    } else {
      rootItems = this.allFiles.filter(f => f.parent_id === this.currentFolderId);
    }

    // Clean Empty State
    if (rootItems.length === 0) {
      container.innerHTML = `
        <div style="flex:1; display:flex; flex-direction:column; align-items:center; justify-content:center; padding:40px; text-align:center; color:var(--text-muted);">
          <div style="font-size:52px; margin-bottom:14px; opacity:0.8;">📁</div>
          <div style="font-size:16px; font-weight:700; color:var(--text-main); margin-bottom:6px;">${t('empty_folder')}</div>
          <div style="font-size:12px; max-width:380px; line-height:1.5; margin-bottom:20px;">${t('empty_folder_desc')}</div>
          <div style="display:flex; gap:10px; flex-wrap:wrap; justify-content:center;">
            <button class="mac-btn primary" onclick="DokuFinder.openNewFileDialog()">✨ + ${t('btn_new_doc')}</button>
            <button class="mac-btn" onclick="DokuFinder.openNewDialog('folder')">📁 + ${t('btn_new_folder')}</button>
            <label class="mac-btn" style="cursor:pointer; margin-bottom:0;">
              ⬆️ ${t('btn_upload')}
              <input type="file" style="display:none;" onchange="DokuFinder.handleFileUpload(event)">
            </label>
          </div>
        </div>
      `;
      return;
    }

    let rowsHtml = '';
    const renderNode = (item, level = 0) => {
      const isFolder = item.is_folder || item.extension === 'folder';
      const isExpanded = this.expandedFolders.has(item.id);
      const isSelected = this.selectedFileId === item.id;
      const icon = this.getFileIcon(item.extension, isFolder);
      const sizeStr = isFolder ? '--' : this.formatSize(item.size_bytes);
      const dateStr = this.formatDate(item.updated_at);
      const kindStr = this.getKindLabel(item.extension, isFolder);
      const padLeft = 12 + level * 20;

      const hasChildren = isFolder && this.allFiles.some(f => f.parent_id === item.id);
      const chevronHtml = hasChildren
        ? `<span class="tree-chevron ${isExpanded ? 'expanded' : ''}" style="width:14px; text-align:center; cursor:pointer; color:var(--text-muted); display:inline-flex; align-items:center; justify-content:center;" onclick="DokuFinder.toggleFolderExpand('${item.id}', event)" title="${isExpanded ? 'Collapse' : 'Expand'}"><svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" style="transition:transform 0.12s; transform:${isExpanded ? 'rotate(90deg)' : 'none'};"><polyline points="9 18 15 12 9 6"></polyline></svg></span>`
        : `<span style="display:inline-block; width:14px;"></span>`;

      rowsHtml += `
        <tr class="${isSelected ? 'selected' : ''}" 
            onclick="DokuFinder.selectFile('${item.id}', event)" 
            ondblclick="DokuFinder.handleDoubleClick('${item.id}')"
            oncontextmenu="DokuFinder.handleContextMenu('${item.id}', event)"
            data-id="${item.id}">
          <td style="padding-left:${padLeft}px; display:flex; align-items:center; gap:6px;">
            ${chevronHtml}
            <span style="font-size:15px; margin-right:4px; display:inline-flex; align-items:center;">${icon}</span>
            <span style="font-weight:500; text-overflow:ellipsis; overflow:hidden; white-space:nowrap;">${item.name}</span>
            ${item.is_shared_public ? '<span style="font-size:9px; background:#10b981; color:#fff; border-radius:4px; padding:1px 4px; margin-left:4px;">🌐</span>' : ''}
            ${item.is_shared_internal ? '<span style="font-size:9px; background:#3b82f6; color:#fff; border-radius:4px; padding:1px 4px; margin-left:4px;">👥</span>' : ''}
          </td>
          <td class="file-item-meta" style="color:var(--text-muted); font-size:11px;">${dateStr}</td>
          <td class="file-item-meta" style="color:var(--text-muted); font-size:11px; text-align:right; padding-right:20px;">${sizeStr}</td>
          <td class="file-item-meta" style="color:var(--text-muted); font-size:11px;">${kindStr}</td>
        </tr>
      `;

      if (isFolder && isExpanded && !this.searchQuery) {
        const children = this.allFiles.filter(f => f.parent_id === item.id);
        children.forEach(child => renderNode(child, level + 1));
      }
    };

    rootItems.forEach(item => renderNode(item, 0));

    container.innerHTML = `
      <div class="finder-table-container">
        <table class="finder-list-table">
          <thead>
            <tr>
              <th style="width:48%;">${t('col_name')}</th>
              <th style="width:24%;">${t('col_date')}</th>
              <th style="width:12%; text-align:right; padding-right:20px;">${t('col_size')}</th>
              <th style="width:16%;">${t('col_kind')}</th>
            </tr>
          </thead>
          <tbody>
            ${rowsHtml}
          </tbody>
        </table>
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     2. Icons View
     ------------------------------------------------------------------------ */
  renderIconsView(container) {
    const t = (k) => window.DokuI18n ? window.DokuI18n.t(k) : k;
    const items = this.allFiles.filter(f => f.parent_id === this.currentFolderId);

    if (items.length === 0) {
      container.innerHTML = `
        <div style="flex:1; display:flex; flex-direction:column; align-items:center; justify-content:center; padding:40px; text-align:center; color:var(--text-muted);">
          <div style="font-size:52px; margin-bottom:14px; opacity:0.8;">📁</div>
          <div style="font-size:16px; font-weight:700; color:var(--text-main); margin-bottom:6px;">${t('empty_folder')}</div>
          <div style="font-size:12px; max-width:380px; line-height:1.5; margin-bottom:20px;">${t('empty_folder_desc')}</div>
          <div style="display:flex; gap:10px;">
            <button class="mac-btn primary" onclick="DokuFinder.openNewFileDialog()">✨ + ${t('btn_new_doc')}</button>
            <button class="mac-btn" onclick="DokuFinder.openNewDialog('folder')">📁 + ${t('btn_new_folder')}</button>
          </div>
        </div>
      `;
      return;
    }

    container.innerHTML = `
      <div class="file-grid" style="padding:20px; overflow-y:auto; flex:1;">
        ${items.map(file => {
      const isFolder = file.is_folder || file.extension === 'folder';
      const isSelected = this.selectedFileId === file.id;
      const icon = this.getFileIcon(file.extension, isFolder);
      const isImg = !isFolder && ['jpg', 'jpeg', 'png', 'bmp', 'webp', 'gif', 'svg'].includes(file.extension);
      const imgSrc = isImg ? (file.content && (file.content.startsWith('data:') || file.content.startsWith('http')) ? file.content : `/api/files/${file.id}/download`) : null;
      const visualIcon = (isImg && imgSrc)
        ? `<img src="${imgSrc}" style="width:48px; height:48px; object-fit:cover; border-radius:6px; box-shadow:0 2px 6px rgba(0,0,0,0.18);" onerror="this.outerHTML='${icon.replace(/'/g, "\\'")}';">`
        : icon;

      return `
            <div class="file-card ${isSelected ? 'selected' : ''}" 
                 onclick="DokuFinder.selectFile('${file.id}', event)" 
                 ondblclick="DokuFinder.handleDoubleClick('${file.id}')"
                 oncontextmenu="DokuFinder.handleContextMenu('${file.id}', event)">
              ${file.is_shared_public ? '<div class="file-badge-share">🌐</div>' : ''}
              ${file.is_shared_internal ? '<div class="file-badge-share" style="background:#3b82f6;">👥</div>' : ''}
              <div class="file-icon-box">${visualIcon}</div>
              <div class="file-name">${file.name}</div>
            </div>
          `;
    }).join('')}
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     3. Columns View (Miller Columns)
     ------------------------------------------------------------------------ */
  renderColumnsView(container) {
    const rootFiles = this.allFiles.filter(f => f.parent_id === 'root');
    const selectedFile = this.allFiles.find(f => f.id === this.selectedFileId);
    let col2Files = [];
    if (selectedFile && (selectedFile.is_folder || selectedFile.extension === 'folder')) {
      col2Files = this.allFiles.filter(f => f.parent_id === selectedFile.id);
    }

    container.innerHTML = `
      <div class="finder-columns-container">
        <!-- Col 1: Root -->
        <div class="miller-column">
          ${rootFiles.length > 0 ? rootFiles.map(f => {
      const isF = f.is_folder || f.extension === 'folder';
      const isSel = this.selectedFileId === f.id;
      return `
              <div class="miller-item ${isSel ? 'selected' : ''}" onclick="DokuFinder.selectFile('${f.id}', event)" ondblclick="DokuFinder.handleDoubleClick('${f.id}')">
                <span style="display:flex; align-items:center; gap:6px;">
                  <span>${this.getFileIcon(f.extension, isF)}</span>
                  <span>${f.name}</span>
                </span>
                ${isF ? '<span style="font-size:10px; opacity:0.6;">›</span>' : ''}
              </div>
            `;
    }).join('') : `<div style="padding:20px; font-size:11px; color:var(--text-muted); text-align:center;">Empty</div>`}
        </div>

        <!-- Col 2: Subfolder -->
        <div class="miller-column">
          ${col2Files.length > 0 ? col2Files.map(f => {
      const isF = f.is_folder || f.extension === 'folder';
      const isSel = this.selectedFileId === f.id;
      return `
              <div class="miller-item ${isSel ? 'selected' : ''}" onclick="DokuFinder.selectFile('${f.id}', event)" ondblclick="DokuFinder.handleDoubleClick('${f.id}')">
                <span style="display:flex; align-items:center; gap:6px;">
                  <span>${this.getFileIcon(f.extension, isF)}</span>
                  <span>${f.name}</span>
                </span>
                ${isF ? '<span style="font-size:10px; opacity:0.6;">›</span>' : ''}
              </div>
            `;
    }).join('') : `<div style="padding:20px; font-size:11px; color:var(--text-muted); text-align:center;">Select folder to view contents</div>`}
        </div>

        <!-- Col 3: Preview Pane -->
        <div class="miller-preview-pane">
          ${selectedFile ? (() => {
            const isImg = !selectedFile.is_folder && ['jpg', 'jpeg', 'png', 'bmp', 'webp', 'gif', 'svg'].includes(selectedFile.extension);
            const imgSrc = isImg ? (selectedFile.content && (selectedFile.content.startsWith('data:') || selectedFile.content.startsWith('http')) ? selectedFile.content : `/api/files/${selectedFile.id}/download`) : null;
            return `
              ${isImg && imgSrc 
                ? `<img src="${imgSrc}" style="max-height:140px; max-width:90%; object-fit:contain; border-radius:6px; box-shadow:0 4px 12px rgba(0,0,0,0.18); margin-bottom:12px;">` 
                : `<div style="font-size:54px; margin-bottom:12px;">${this.getFileIcon(selectedFile.extension, selectedFile.is_folder)}</div>`}
              <div style="font-weight:700; font-size:14px; margin-bottom:6px;">${selectedFile.name}</div>
              <div style="font-size:12px; color:var(--text-muted); margin-bottom:12px;">${this.getKindLabel(selectedFile.extension, selectedFile.is_folder)} • ${this.formatSize(selectedFile.size_bytes)}</div>
              <button class="mac-btn primary" onclick="DokuFinder.openFile('${selectedFile.id}')">${DokuI18n.lang === 'id' ? 'Buka Berkas' : 'Open File'}</button>
              ${isImg ? `
                <button class="mac-btn" onclick="DokuFinder.setFileAsWallpaper('${selectedFile.id}')" style="margin-top:6px; font-size:11.5px;">
                  🌄 ${DokuI18n.lang === 'id' ? 'Jadikan Wallpaper' : 'Set as Wallpaper'}
                </button>
              ` : ''}
            `;
          })() : `<div style="color:var(--text-muted); font-size:12px;">Select an item to preview</div>`}
        </div>
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     4. Gallery View
     ------------------------------------------------------------------------ */
  renderGalleryView(container) {
    const items = this.allFiles.filter(f => f.parent_id === this.currentFolderId);
    const selected = this.allFiles.find(f => f.id === this.selectedFileId) || items[0];
    const isId = (DokuI18n.lang === 'id');

    container.innerHTML = `
      <div class="finder-gallery-container">
        <!-- Hero Preview -->
        <div class="gallery-preview-hero">
          ${selected ? (() => {
            const isImg = !selected.is_folder && ['jpg', 'jpeg', 'png', 'bmp', 'webp', 'gif', 'svg'].includes(selected.extension);
            const imgSrc = isImg ? (selected.content && (selected.content.startsWith('data:') || selected.content.startsWith('http')) ? selected.content : `/api/files/${selected.id}/download`) : null;
            return `
              ${isImg && imgSrc 
                ? `<img src="${imgSrc}" style="max-height:220px; max-width:85%; object-fit:contain; border-radius:8px; box-shadow:0 8px 24px rgba(0,0,0,0.3); margin-bottom:14px;">` 
                : `<div style="font-size:64px; margin-bottom:12px;">${this.getFileIcon(selected.extension, selected.is_folder)}</div>`}
              <h2 style="font-size:18px; font-weight:700; margin-bottom:6px;">${selected.name}</h2>
              <div style="font-size:12px; color:var(--text-muted); margin-bottom:16px;">
                ${this.getKindLabel(selected.extension, selected.is_folder)} • ${this.formatSize(selected.size_bytes)} • Modified ${this.formatDate(selected.updated_at)}
              </div>
              <div style="display:flex; gap:8px;">
                <button class="mac-btn primary" onclick="DokuFinder.openFile('${selected.id}')">${isId ? 'Buka Dokumen' : 'Open Document'}</button>
                ${isImg ? `
                  <button class="mac-btn" onclick="DokuFinder.setFileAsWallpaper('${selected.id}')">
                    🌄 ${isId ? 'Jadikan Wallpaper' : 'Set as Wallpaper'}
                  </button>
                ` : ''}
              </div>
            `;
          })() : `<div style="color:var(--text-muted);">No items selected</div>`}
        </div>

        <!-- Carousel Strip -->
        <div class="gallery-carousel-strip">
          ${items.map(f => {
            const isImg = !f.is_folder && ['jpg', 'jpeg', 'png', 'bmp', 'webp', 'gif', 'svg'].includes(f.extension);
            const imgSrc = isImg ? (f.content && (f.content.startsWith('data:') || f.content.startsWith('http')) ? f.content : `/api/files/${f.id}/download`) : null;
            return `
              <div class="gallery-thumb-card ${selected && selected.id === f.id ? 'selected' : ''}" onclick="DokuFinder.selectFile('${f.id}', event)">
                ${isImg && imgSrc 
                  ? `<img src="${imgSrc}" style="width:32px; height:32px; object-fit:cover; border-radius:4px;">` 
                  : `<div style="font-size:24px;">${this.getFileIcon(f.extension, f.is_folder)}</div>`}
                <div style="font-size:10px; max-width:54px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; margin-top:2px;">${f.name}</div>
              </div>
            `;
          }).join('')}
        </div>
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     Interaction Handlers
     ------------------------------------------------------------------------ */
  toggleFolderExpand(folderId, event) {
    if (event) event.stopPropagation();
    if (this.expandedFolders.has(folderId)) {
      this.expandedFolders.delete(folderId);
    } else {
      this.expandedFolders.add(folderId);
    }
    this.renderCurrentView();
  },

  selectFile(fileId, event) {
    if (event) event.stopPropagation();
    this.selectedFileId = fileId;
    this.renderCurrentView();
  },

  handleDoubleClick(fileId) {
    const file = this.allFiles.find(f => f.id === fileId);
    if (!file) return;

    if (file.is_folder || file.extension === 'folder') {
      this.navigateToFolder(file.id, file.name);
    } else {
      this.openFile(fileId);
    }
  },

  navigateToDocuments() {
    const fld = this.allFiles.find(f => (f.is_folder || f.extension === 'folder') && (f.id === 'fld_docs' || f.id.startsWith('fld_docs') || f.name.toLowerCase() === 'documents' || f.name.toLowerCase() === 'dokumen'));
    const id = fld ? fld.id : 'fld_docs';
    const name = fld ? fld.name : (DokuI18n.lang === 'id' ? 'Dokumen' : 'Documents');
    this.navigateToFolder(id, name);
  },

  navigateToProjects() {
    const fld = this.allFiles.find(f => (f.is_folder || f.extension === 'folder') && (f.id === 'fld_projects' || f.id.startsWith('fld_projects') || f.name.toLowerCase() === 'projects' || f.name.toLowerCase() === 'proyek'));
    const id = fld ? fld.id : 'fld_projects';
    const name = fld ? fld.name : (DokuI18n.lang === 'id' ? 'Proyek' : 'Projects');
    this.navigateToFolder(id, name);
  },

  navigateToFolder(folderId, folderName) {
    this.currentCategory = 'all';
    this.currentFolderId = folderId;
    this.folderHistory = this.folderHistory.slice(0, this.historyIndex + 1);
    this.folderHistory.push(folderId);
    this.historyIndex = this.folderHistory.length - 1;

    // Update active highlight on sidebar
    document.querySelectorAll('.finder-sidebar .sidebar-item').forEach(el => el.classList.remove('active'));
    if (folderId === 'root') {
      const el = document.getElementById('sidebar-item-all');
      if (el) el.classList.add('active');
    } else if (folderId === 'fld_docs' || folderName === 'Documents' || folderName === 'Dokumen') {
      const el = document.getElementById('sidebar-item-docs');
      if (el) el.classList.add('active');
    } else if (folderId === 'fld_projects' || folderName === 'Projects' || folderName === 'Proyek') {
      const el = document.getElementById('sidebar-item-projects');
      if (el) el.classList.add('active');
    }

    this.updateBreadcrumbs();
    this.updateNavButtons();
    this.renderCurrentView();
  },

  updateBreadcrumbs() {
    const el = document.getElementById('finder-breadcrumbs');
    if (!el) return;
    const userName = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.name : 'dionipe';
    if (this.currentCategory === 'recents') {
      el.innerHTML = `<span class="crumb active" style="font-weight:700;">🕒 ${DokuI18n.lang === 'id' ? 'Berkas Terbaru' : 'Recent Files'}</span>`;
      return;
    }
    if (this.currentCategory === 'shared') {
      el.innerHTML = `<span class="crumb active" style="font-weight:700;">👥 ${DokuI18n.lang === 'id' ? 'Dibagikan dengan Saya' : 'Shared with Me'}</span>`;
      return;
    }
    if (this.currentCategory === 'public') {
      el.innerHTML = `<span class="crumb active" style="font-weight:700;">🌐 ${DokuI18n.lang === 'id' ? 'Tautan Berbagi Publik' : 'Public Shared Links'}</span>`;
      return;
    }
    if (this.currentCategory === 'trash') {
      el.innerHTML = `<span class="crumb active" style="font-weight:700;">🗑️ ${DokuI18n.lang === 'id' ? 'Tempat Sampah' : 'Trash'}</span>`;
      return;
    }

    if (this.currentFolderId === 'root') {
      el.innerHTML = `<span class="crumb active" onclick="DokuFinder.navigateToFolder('root', '${userName}')" style="cursor:pointer; font-weight:700;">🏠 ${DokuI18n.lang === 'id' ? 'Semua Berkas' : 'All Files'} (${userName})</span>`;
      return;
    }
    let crumbs = [];
    let curId = this.currentFolderId;
    while (curId && curId !== 'root') {
      const f = this.allFiles.find(x => x.id === curId);
      if (f) {
        crumbs.unshift(f);
        curId = f.parent_id;
      } else {
        const fallbackName = curId === 'fld_docs' ? (DokuI18n.lang === 'id' ? 'Dokumen' : 'Documents') : (curId === 'fld_projects' ? (DokuI18n.lang === 'id' ? 'Proyek' : 'Projects') : curId);
        crumbs.unshift({ id: curId, name: fallbackName, parent_id: 'root' });
        break;
      }
    }
    let html = `<span class="crumb" onclick="DokuFinder.navigateToFolder('root', '${userName}')" style="cursor:pointer; color:var(--text-muted);">🏠 ${DokuI18n.lang === 'id' ? 'Semua Berkas' : 'All Files'}</span>`;
    crumbs.forEach((c, idx) => {
      const isLast = idx === crumbs.length - 1;
      html += ` <span style="opacity:0.35; font-size:10px;">›</span> <span class="crumb ${isLast ? 'active' : ''}" onclick="DokuFinder.navigateToFolder('${c.id}', '${c.name}')" style="cursor:pointer; ${isLast ? 'color:var(--text-main); font-weight:700;' : 'color:var(--text-muted);'}">📁 ${c.name}</span>`;
    });
    el.innerHTML = html;
  },

  navBack() {
    if (this.historyIndex > 0) {
      this.historyIndex--;
      this.currentFolderId = this.folderHistory[this.historyIndex];
      this.updateBreadcrumbs();
      this.updateNavButtons();
      this.renderCurrentView();
    }
  },

  navForward() {
    if (this.historyIndex < this.folderHistory.length - 1) {
      this.historyIndex++;
      this.currentFolderId = this.folderHistory[this.historyIndex];
      this.updateBreadcrumbs();
      this.updateNavButtons();
      this.renderCurrentView();
    }
  },

  updateNavButtons() {
    const btnBack = document.getElementById('btn-finder-back');
    const btnFwd = document.getElementById('btn-finder-forward');
    if (btnBack) btnBack.disabled = this.historyIndex <= 0;
    if (btnFwd) btnFwd.disabled = this.historyIndex >= this.folderHistory.length - 1;
  },

  setView(viewName) {
    this.currentView = viewName;
    document.querySelectorAll('.finder-segment-btn').forEach(btn => btn.classList.remove('active'));
    this.renderCurrentView();
  },

  setCategory(cat) {
    this.currentCategory = cat;
    this.searchQuery = '';
    if (cat === 'all') {
      this.currentFolderId = 'root';
    }
    this.updateBreadcrumbs();
    document.querySelectorAll('.finder-sidebar .sidebar-item').forEach(el => el.classList.remove('active'));
    this.loadAllFiles();
  },

  filterByTag(tagName) {
    this.currentCategory = 'all';
    this.searchQuery = '';
    this.renderCurrentView();
  },

  handleSearch(query) {
    this.searchQuery = query.trim();
    this.renderCurrentView();
  },

  updateToolbarSelection() {
    const sel = this.allFiles.find(f => f.id === this.selectedFileId);
    const isFolder = sel && (sel.is_folder || sel.extension === 'folder');
    const btnShare = document.getElementById('btn-quick-share');
    const btnRename = document.getElementById('btn-quick-rename');
    const btnDelete = document.getElementById('btn-quick-delete');
    const btnDownload = document.getElementById('btn-quick-download');
    const btnQr = document.getElementById('btn-quick-qr');
    if (btnShare) btnShare.disabled = !sel;
    if (btnRename) btnRename.disabled = !sel;
    if (btnDelete) btnDelete.disabled = !sel;
    if (btnDownload) btnDownload.disabled = !sel || isFolder;
    if (btnQr) btnQr.disabled = !sel || isFolder;
  },

  /* ------------------------------------------------------------------------
     File Renaming
     ------------------------------------------------------------------------ */
  openRenameModal() {
    if (!this.selectedFileId) return;
    const file = this.allFiles.find(f => f.id === this.selectedFileId);
    if (!file) return;

    const modal = document.getElementById('rename-modal-overlay');
    const input = document.getElementById('rename-file-input');
    if (modal && input) {
      input.value = file.name;
      modal.style.display = 'flex';
      input.focus();
      // Select base name without extension for regular files
      const dotIdx = file.name.lastIndexOf('.');
      if (dotIdx > 0 && !file.is_folder && file.extension !== 'folder') {
        input.setSelectionRange(0, dotIdx);
      } else {
        input.select();
      }
    }
  },

  closeRenameModal() {
    const modal = document.getElementById('rename-modal-overlay');
    if (modal) modal.style.display = 'none';
  },

  async submitRename() {
    if (!this.selectedFileId) return;
    const file = this.allFiles.find(f => f.id === this.selectedFileId);
    if (!file) return;

    const input = document.getElementById('rename-file-input');
    const newName = input ? input.value.trim() : '';
    if (!newName || newName === file.name) {
      this.closeRenameModal();
      return;
    }

    try {
      const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : 'usr_dionipe_849';
      const currentTenantId = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : (window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.tenant_id : '');

      const res = await fetch(`/api/files/${file.id}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'X-Tenant-Id': currentTenantId,
          'X-User-Id': currentUserId
        },
        body: JSON.stringify({
          id: file.id,
          name: newName
        })
      });

      const data = await res.json();
      if (res.ok && data.success) {
        file.name = newName;
        if (!file.is_folder && newName.includes('.')) {
          file.extension = newName.split('.').pop().toLowerCase();
        }
        this.closeRenameModal();
        await this.loadAllFiles();
      } else {
        alert((window.DokuI18n ? DokuI18n.t('rename_err') : 'Failed to rename: ') + (data.error || 'Unknown error'));
      }
    } catch (err) {
      alert((window.DokuI18n ? DokuI18n.t('rename_err') : 'Failed to rename: ') + err);
    }
  },

  /* ------------------------------------------------------------------------
     Context Menu & Quick Actions
     ------------------------------------------------------------------------ */
  handleContextMenu(fileId, event) {
    if (event) {
      event.preventDefault();
      event.stopPropagation();
    }
    this.selectFile(fileId, event);
    this.showContextMenu(event.clientX, event.clientY);
  },

  toggleActionMenu(event) {
    if (event) event.stopPropagation();
    const btn = event ? event.currentTarget : document.querySelector('button[title="Actions"]');
    if (!btn) return;
    const rect = btn.getBoundingClientRect();
    this.showContextMenu(rect.left, rect.bottom + 6);
  },

  showContextMenu(x, y) {
    let menu = document.getElementById('finder-context-menu');
    if (!menu) {
      menu = document.createElement('div');
      menu.id = 'finder-context-menu';
      menu.style.position = 'fixed';
      menu.style.zIndex = '9999999';
      menu.style.background = 'var(--window-bg)';
      menu.style.backdropFilter = 'blur(30px)';
      menu.style.border = '1px solid var(--window-border)';
      menu.style.borderRadius = '10px';
      menu.style.boxShadow = '0 12px 30px rgba(0,0,0,0.35)';
      menu.style.padding = '6px';
      menu.style.minWidth = '180px';
      menu.style.fontSize = '12px';
      document.body.appendChild(menu);

      window.addEventListener('click', () => {
        menu.style.display = 'none';
      });
    }

    const sel = this.allFiles.find(f => f.id === this.selectedFileId);
    const isFolder = sel && (sel.is_folder || sel.extension === 'folder');
    const isId = (DokuI18n.lang === 'id');

    menu.innerHTML = `
      ${sel ? `
        <div class="finder-menu-item" onclick="DokuFinder.openFile('${sel.id}')" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
          <span>📂</span> <strong>${isId ? 'Buka' : 'Open'}</strong>
        </div>
        ${!isFolder && ['jpg', 'jpeg', 'png', 'bmp', 'webp', 'gif', 'svg'].includes(sel.extension) ? `
          <div class="finder-menu-item" onclick="DokuFinder.setFileAsWallpaper('${sel.id}')" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px; color:var(--accent-color); font-weight:600;">
            <span>🌄</span> <span>${isId ? 'Jadikan Wallpaper Desktop' : 'Set as Desktop Wallpaper'}</span>
          </div>
        ` : ''}
        <div class="finder-menu-item" onclick="DokuFinder.openRenameModal()" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
          <span>✏️</span> <span>${isId ? 'Ganti Nama' : 'Rename'}</span> <span style="margin-left:auto; font-size:10px; opacity:0.6;">Enter</span>
        </div>
        ${!isFolder ? `
          <div class="finder-menu-item" onclick="DokuFinder.downloadSelected()" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
            <span>⬇️</span> <span>${isId ? 'Unduh Berkas' : 'Download File'}</span>
          </div>
          <div class="finder-menu-item" onclick="DokuFinder.openDownloadQrModal()" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
            <span>📱</span> <span>${isId ? 'QR Code Download' : 'QR Code Download'}</span>
          </div>
        ` : ''}
        <div class="finder-menu-item" onclick="DokuFinder.openShareModal()" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
          <span>↗️</span> <span>${isId ? 'Bagikan' : 'Share'}</span>
        </div>
        <div style="height:1px; background:var(--window-border); margin:4px 0;"></div>
        <div class="finder-menu-item" onclick="DokuFinder.deleteSelected()" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px; color:#ef4444;">
          <span>🗑️</span> <span>${isId ? 'Hapus' : 'Delete'}</span>
        </div>
        <div style="height:1px; background:var(--window-border); margin:4px 0;"></div>
      ` : ''}
      <div class="finder-menu-item" onclick="DokuFinder.openNewFileDialog()" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>✨</span> <span>${isId ? 'Dokumen Baru' : 'New Document'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuFinder.openNewDialog('folder')" style="padding:6px 10px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📁</span> <span>${isId ? 'Folder Baru' : 'New Folder'}</span>
      </div>
    `;

    // Ensure menu stays within screen bounds
    const screenW = window.innerWidth;
    const screenH = window.innerHeight;
    const posX = (x + 190 > screenW) ? screenW - 200 : x;
    const posY = (y + 220 > screenH) ? screenH - 230 : y;

    menu.style.left = `${posX}px`;
    menu.style.top = `${posY}px`;
    menu.style.display = 'block';
  },

  downloadSelected() {
    if (!this.selectedFileId) return;
    const f = this.allFiles.find(x => x.id === this.selectedFileId);
    if (!f || f.is_folder || f.extension === 'folder') return;
    window.open(`/api/files/${f.id}/download`, '_blank');
  },

  setFileAsWallpaper(fileId) {
    const file = this.allFiles.find(f => f.id === fileId);
    if (!file) return;
    const isId = (DokuI18n.lang === 'id');
    const src = file.content && (file.content.startsWith('data:') || file.content.startsWith('http'))
      ? file.content
      : `/api/files/${file.id}/download`;

    DokuHub.setWallpaper(src, true);
    alert(isId ? `✅ "${file.name}" berhasil dijadikan wallpaper desktop!` : `✅ "${file.name}" set as desktop wallpaper!`);
  },

  openFile(fileId) {
    const file = this.allFiles.find(f => f.id === fileId);
    if (!file) return;

    if (file.is_folder || file.extension === 'folder') {
      this.toggleFolderExpand(fileId);
      return;
    }

    switch (file.extension) {
      case 'docx':
        DokuHub.openApp('wordcraft', file);
        break;
      case 'xlsx':
      case 'csv':
      case 'tsv':
        DokuHub.openApp('griidcraft', file);
        break;
      case 'pptx':
        DokuHub.openApp('deckcraft', file);
        break;
      case 'pdf':
        DokuHub.openApp('pdfviewer', file);
        break;
      case 'jpg':
      case 'jpeg':
      case 'png':
      case 'bmp':
      case 'webp':
      case 'gif':
      case 'svg':
        DokuHub.openApp('preview', file);
        break;
      case 'notes':
      case 'md':
      case 'go':
      case 'swift':
      case 'sh':
      case 'mod':
        DokuHub.openApp('notes', file);
        break;
      default:
        alert(`Opening: ${file.name}\nKind: ${this.getKindLabel(file.extension, false)}`);
    }
  },

  /* ------------------------------------------------------------------------
     File Creation & Uploading
     ------------------------------------------------------------------------ */
  _selectedDocType: 'docx',
  _userHasCustomName: false,

  openNewFileDialog() {
    this._userHasCustomName = false;
    this._selectedDocType = 'docx';
    const modal = document.getElementById('new-file-modal-overlay');
    if (modal) modal.style.display = 'flex';

    // Reset cards active state
    document.querySelectorAll('.new-type-card').forEach(c => {
      const isDocx = c.getAttribute('data-ext') === 'docx';
      c.style.borderColor = isDocx ? 'var(--accent-color)' : 'var(--window-border)';
      c.classList.toggle('active', isDocx);
    });

    const input = document.getElementById('new-file-name-input');
    if (input) {
      const isId = (DokuI18n.lang === 'id');
      input.value = isId ? 'Dokumen Baru' : 'New Document';
      input.focus();
      input.select();
      input.oninput = () => {
        this._userHasCustomName = true;
      };
    }
  },

  openNewDialog(type) {
    this.openNewFileDialog();
    const card = document.querySelector(`.new-type-card[data-ext="${type}"]`);
    if (card) {
      this.selectDocType(card, type);
      const input = document.getElementById('new-file-name-input');
      if (input) input.select();
    }
  },

  closeNewFileDialog() {
    const modal = document.getElementById('new-file-modal-overlay');
    if (modal) modal.style.display = 'none';
    this._userHasCustomName = false;
  },

  selectDocType(cardEl, ext) {
    this._selectedDocType = ext;
    document.querySelectorAll('.new-type-card').forEach(c => {
      c.style.borderColor = 'var(--window-border)';
      c.classList.remove('active');
    });
    cardEl.style.borderColor = 'var(--accent-color)';
    cardEl.classList.add('active');

    const input = document.getElementById('new-file-name-input');
    if (input) {
      const isId = (DokuI18n.lang === 'id');
      const defaultSamples = [
        'Dokumen Baru', 'New Document',
        'Rencana Strategis 2027', 'Strategic Project Plan 2027',
        'Anggaran & Estimasi Finansial', 'Financial Budget & Estimation',
        'Pitch Deck & Materi Presentasi', 'Pitch Deck & Presentation Slides',
        'Dokumentasi Markdown', 'Markdown Document',
        'Folder Proyek Baru', 'New Project Folder'
      ];
      // Only set sample name if user hasn't explicitly typed a custom name!
      if (!this._userHasCustomName || defaultSamples.includes(input.value.trim()) || !input.value.trim()) {
        const sampleNames = {
          docx: isId ? 'Rencana Strategis 2027' : 'Strategic Project Plan 2027',
          xlsx: isId ? 'Anggaran & Estimasi Finansial' : 'Financial Budget & Estimation',
          pptx: isId ? 'Pitch Deck & Materi Presentasi' : 'Pitch Deck & Presentation Slides',
          md: isId ? 'Dokumentasi Markdown' : 'Markdown Document',
          folder: isId ? 'Folder Proyek Baru' : 'New Project Folder'
        };
        input.value = sampleNames[ext] || (isId ? 'Dokumen Baru' : 'New Document');
        input.select();
      }
    }
  },

  async submitNewFile() {
    const input = document.getElementById('new-file-name-input');
    let name = input ? input.value.trim() : '';
    if (!name) name = DokuI18n.lang === 'id' ? 'Dokumen Baru' : 'New Document';

    const ext = this._selectedDocType;
    const isFolder = (ext === 'folder');

    if (!isFolder) {
      // Remove any trailing known extension if user changed type
      const knownExts = ['docx', 'xlsx', 'pptx', 'pdf', 'txt', 'md'];
      const lastDot = name.lastIndexOf('.');
      if (lastDot > 0) {
        const curExt = name.substring(lastDot + 1).toLowerCase();
        if (knownExts.includes(curExt)) {
          name = name.substring(0, lastDot);
        }
      }
      name = `${name}.${ext}`;
    }

    try {
      const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : 'usr_dionipe_849';
      const currentTenantId = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : (window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.tenant_id : '');
      const res = await fetch('/api/files', {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'X-Tenant-Id': currentTenantId,
          'X-User-Id': currentUserId
        },
        body: JSON.stringify({
          name,
          extension: isFolder ? 'folder' : ext,
          parent_id: this.currentFolderId,
          owner_id: currentUserId,
          tenant_id: currentTenantId,
          is_folder: isFolder
        })
      });
      const data = await res.json();
      if (data.success) {
        this.closeNewFileDialog();
        await this.loadAllFiles();
        if (!isFolder) {
          this.openFile(data.id);
        }
      } else {
        alert("Failed to create file: " + (data.error || 'Unknown error'));
      }
    } catch (err) {
      alert("Error: " + err);
    }
  },

  async handleFileUpload(e) {
    const files = e.target.files;
    if (!files || files.length === 0) return;

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const ext = (file.name.split('.').pop() || '').toLowerCase();
      const isTextFile = ['txt', 'md', 'markdown', 'csv', 'json', 'html', 'htm', 'css', 'js', 'xml', 'log', 'yaml', 'yml'].includes(ext);
      const isBinary = !isTextFile;

      const reader = new FileReader();

      reader.onload = async (event) => {
        const content = event.target.result;
        try {
          const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : 'usr_dionipe_849';
          const currentTenantId = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : (window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.tenant_id : '');
          const res = await fetch('/api/files', {
            method: 'POST',
            headers: { 
              'Content-Type': 'application/json',
              'X-Tenant-Id': currentTenantId,
              'X-User-Id': currentUserId
            },
            body: JSON.stringify({
              name: file.name,
              extension: ext,
              content: content,
              owner_id: currentUserId,
              tenant_id: currentTenantId,
              parent_id: this.currentFolderId,
              size: file.size
            })
          });
          const data = await res.json();
          if (data.success) {
            await this.loadAllFiles();
          }
        } catch (err) {
          alert("Failed to upload file: " + err);
        }
      };

      if (isBinary) {
        reader.readAsDataURL(file);
      } else {
        reader.readAsText(file);
      }
    }
    e.target.value = '';
  },

  async deleteSelected() {
    if (!this.selectedFileId) return;
    const item = this.allFiles.find(f => f.id === this.selectedFileId);
    if (!item) return;

    const confirmMsg = DokuI18n.lang === 'id'
      ? `Hapus "${item.name}" secara permanen dari BijiOS?`
      : `Delete "${item.name}" permanently from BijiOS?`;

    if (!confirm(confirmMsg)) return;

    try {
      await fetch(`/api/files/${this.selectedFileId}`, { method: 'DELETE' });
      this.selectedFileId = null;
      await this.loadAllFiles();
    } catch (err) {
      alert("Failed to delete file: " + err);
    }
  },

  /* ------------------------------------------------------------------------
     Sharing System
     ------------------------------------------------------------------------ */
  async openShareModal() {
    const item = this.allFiles.find(f => f.id === this.selectedFileId);
    if (!item) {
      alert(DokuI18n.lang === 'id' ? "Silakan pilih berkas yang ingin dibagikan terlebih dahulu." : "Please select a file to share first.");
      return;
    }
    const modal = document.getElementById('share-modal-overlay');
    const title = document.getElementById('share-modal-title');
    if (title) title.textContent = `${DokuI18n.lang === 'id' ? 'Bagikan' : 'Share'}: ${item.name}`;

    // Load real users dynamically
    const sel = document.getElementById('share-internal-user');
    if (sel) {
      try {
        const res = await fetch('/api/users');
        const users = await res.json();
        const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : '';
        const others = (users || []).filter(u => u.id !== currentUserId);
        if (others.length > 0) {
          sel.innerHTML = others.map(u => `<option value="${u.id}">${u.name} (${u.role})</option>`).join('');
        } else {
          sel.innerHTML = `<option value="">No other users found</option>`;
        }
      } catch (e) { }
    }

    if (modal) modal.style.display = 'flex';
  },

  closeShareModal() {
    const modal = document.getElementById('share-modal-overlay');
    if (modal) modal.style.display = 'none';
  },

  switchShareTab(tab) {
    const pubTab = document.getElementById('share-tab-public');
    const intTab = document.getElementById('share-tab-internal');
    const extTab = document.getElementById('share-tab-external');
    const btnPub = document.getElementById('tab-btn-pub');
    const btnInt = document.getElementById('tab-btn-int');
    const btnExt = document.getElementById('tab-btn-ext');

    if (pubTab) pubTab.style.display = tab === 'public' ? 'block' : 'none';
    if (intTab) intTab.style.display = tab === 'internal' ? 'block' : 'none';
    if (extTab) extTab.style.display = tab === 'external' ? 'block' : 'none';
    if (btnPub) btnPub.classList.toggle('primary', tab === 'public');
    if (btnInt) btnInt.classList.toggle('primary', tab === 'internal');
    if (btnExt) btnExt.classList.toggle('primary', tab === 'external');
  },

  async generatePublicShare() {
    if (!this.selectedFileId) return;
    const pw = document.getElementById('share-public-pw').value;
    const perm = document.getElementById('share-public-perm').value;

    try {
      const res = await fetch('/api/shares', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          file_id: this.selectedFileId,
          share_type: 'public',
          permission: perm,
          password: pw || null
        })
      });
      const data = await res.json();
      if (data.success) {
        const fullUrl = `${window.location.origin}${data.share_url}`;
        const dlUrl = `${window.location.origin}/api/files/${this.selectedFileId}/download`;
        const item = this.allFiles.find(f => f.id === this.selectedFileId);
        this._lastShareUrls = {
          share: fullUrl,
          download: dlUrl,
          fileName: item ? item.name : 'dokumen',
          mode: 'share'
        };
        const resBox = document.getElementById('share-public-result');
        const input = document.getElementById('share-link-input');
        input.value = fullUrl;
        resBox.style.display = 'block';
        this.renderShareQr('share');
        await this.loadAllFiles();
      } else {
        alert("Failed to generate link: " + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  renderShareQr(mode = 'share') {
    if (!this._lastShareUrls) return;
    this._lastShareUrls.mode = mode;
    const url = mode === 'download' ? this._lastShareUrls.download : this._lastShareUrls.share;
    const input = document.getElementById('share-link-input');
    const canvas = document.getElementById('share-qr-canvas');
    const tabLink = document.getElementById('share-qr-tab-link');
    const tabDl = document.getElementById('share-qr-tab-dl');
    if (input) input.value = url;
    if (tabLink) tabLink.classList.toggle('primary', mode === 'share');
    if (tabDl) tabDl.classList.toggle('primary', mode === 'download');
    if (canvas && window.DokuQR) {
      DokuQR.renderCanvas(canvas, url, 150);
    }
  },

  downloadShareQrPng() {
    if (!this._lastShareUrls || !window.DokuQR) return;
    const mode = this._lastShareUrls.mode || 'share';
    const url = mode === 'download' ? this._lastShareUrls.download : this._lastShareUrls.share;
    const caption = `${this._lastShareUrls.fileName} (${mode === 'download' ? 'Download' : 'Share'})`;
    DokuQR.downloadQrPng(url, this._lastShareUrls.fileName, caption);
  },

  openDownloadQrModal() {
    const item = this.allFiles.find(f => f.id === this.selectedFileId);
    if (!item || item.is_folder) {
      alert(DokuI18n.lang === 'id' ? 'Pilih berkas terlebih dahulu untuk membuat QR Code.' : 'Please select a file first.');
      return;
    }
    const dlUrl = `${window.location.origin}/api/files/${item.id}/download`;
    if (window.DokuQR) {
      DokuQR.openModal({
        title: DokuI18n.lang === 'id' ? 'QR Code Download Berkas' : 'File Download QR Code',
        fileName: item.name,
        downloadUrl: dlUrl,
        defaultMode: 'download'
      });
    }
  },

  async copyShareLink() {
    const input = document.getElementById('share-link-input');
    if (!input || !input.value) return;
    const text = input.value;
    let ok = false;
    if (window.DokuQR && typeof DokuQR.copyText === 'function') {
      ok = await DokuQR.copyText(text);
    } else {
      try {
        input.focus();
        input.select();
        ok = document.execCommand('copy');
      } catch {}
    }
    if (ok) {
      if (window.DokuIsland) {
        DokuIsland.info(DokuI18n.lang === 'id' ? 'Tautan berhasil disalin ke clipboard!' : 'Link copied to clipboard!', 'Share');
      } else {
        alert(DokuI18n.lang === 'id' ? 'Tautan berhasil disalin ke clipboard!' : 'Link copied to clipboard!');
      }
    } else {
      input.focus();
      input.select();
      alert(DokuI18n.lang === 'id' ? 'Silakan tekan Ctrl+C / Cmd+C untuk menyalin tautan.' : 'Press Ctrl+C / Cmd+C to copy the link.');
    }
  },

  async saveInternalShare() {
    if (!this.selectedFileId) return;
    const isId = (DokuI18n.lang === 'id');
    const userId = document.getElementById('share-internal-user')?.value || '';
    const perm = document.getElementById('share-internal-perm')?.value || 'view';
    const note = document.getElementById('share-internal-note')?.value || '';

    try {
      const headers = { 'Content-Type': 'application/json' };
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const res = await fetch('/api/shares', {
        method: 'POST',
        headers,
        body: JSON.stringify({
          file_id: this.selectedFileId,
          share_type: 'internal',
          target_user_id: userId,
          permission: perm,
          request_note: note
        })
      });
      const data = await res.json();
      if (data.success) {
        const msg = data.needs_approval
          ? (isId ? 'Pengajuan share internal dikirim! Menunggu persetujuan (Approval) Supervisor.' : 'Internal share request submitted! Awaiting Supervisor approval.')
          : (isId ? 'Akses internal disetujui & langsung aktif!' : 'Internal access granted & active!');
        if (window.DokuIsland) DokuIsland.success(msg, 'Share Approval');
        else alert(msg);
        this.closeShareModal();
        await this.loadAllFiles();
      } else {
        alert("Failed: " + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  async saveExternalShare() {
    if (!this.selectedFileId) return;
    const isId = (DokuI18n.lang === 'id');
    const email = (document.getElementById('share-external-email')?.value || '').trim();
    const perm = document.getElementById('share-external-perm')?.value || 'view';
    const note = (document.getElementById('share-external-note')?.value || '').trim();

    if (!email) {
      alert(isId ? 'Masukkan alamat email penerima eksternal.' : 'Please enter external recipient email.');
      return;
    }

    try {
      const headers = { 'Content-Type': 'application/json' };
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const res = await fetch('/api/shares', {
        method: 'POST',
        headers,
        body: JSON.stringify({
          file_id: this.selectedFileId,
          share_type: 'external',
          target_email: email,
          permission: perm,
          request_note: note
        })
      });
      const data = await res.json();
      if (data.success) {
        const msg = data.needs_approval
          ? (isId ? `Pengajuan share eksternal ke ${email} dikirim ke Supervisor untuk Approval!` : `External share request to ${email} sent to Supervisor for approval!`)
          : (isId ? `Tautan eksternal ke ${email} aktif! URL: ${window.location.origin}${data.share_url}` : `External share link active! URL: ${window.location.origin}${data.share_url}`);
        if (window.DokuIsland) DokuIsland.success(msg, 'Share Approval');
        else alert(msg);
        this.closeShareModal();
        await this.loadAllFiles();
      } else {
        alert("Failed: " + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  /* ------------------------------------------------------------------------
     Formatters & Helpers
     ------------------------------------------------------------------------ */
  getFileIcon(ext, isFolder) {
    if (isFolder) {
      return `<svg width="17" height="17" viewBox="0 0 24 24" fill="none" style="vertical-align:middle; filter:drop-shadow(0 1px 1px rgba(0,0,0,0.3));"><path d="M2 6C2 4.89543 2.89543 4 4 4H9.17157C9.70201 4 10.2107 4.21071 10.5858 4.58579L12 6H20C21.1046 6 22 6.89543 22 8V18C22 19.1046 21.1046 20 20 20H4C2.89543 20 2 19.1046 2 18V6Z" fill="#0284c7"/><path d="M2 9.5H22V18C22 19.1046 21.1046 20 20 20H4C2.89543 20 2 19.1046 2 18V9.5Z" fill="#38bdf8"/></svg>`;
    }
    switch (ext) {
      case 'bin':
      case 'unix':
      case 'exe':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#181818; border-radius:3px; border:1px solid #444; font-family:monospace; font-size:7px; font-weight:800; color:#22c55e;">exec</span>`;
      case 'mod':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#334155; border-radius:3px; font-size:10px; color:#fff;">📄</span>`;
      case 'go':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#00add8; border-radius:3px; font-family:system-ui; font-size:8px; font-weight:800; color:#fff;">GO</span>`;
      case 'md':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#2563eb; border-radius:3px; font-family:system-ui; font-size:7px; font-weight:800; color:#fff;">MD</span>`;
      case 'swift':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#ea580c; border-radius:3px; font-size:9px; color:#fff;">🦅</span>`;
      case 'sh':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#111827; border:1px solid #475569; border-radius:3px; font-family:monospace; font-size:8px; font-weight:800; color:#38bdf8;">&gt;_</span>`;
      case 'png':
      case 'jpg':
      case 'jpeg':
      case 'bmp':
      case 'webp':
      case 'gif':
      case 'svg':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#7c3aed; border-radius:3px; font-size:10px; color:#fff;">🖼️</span>`;
      case 'docx':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#1d4ed8; border-radius:3px; font-family:system-ui; font-size:9px; font-weight:800; color:#fff;">W</span>`;
      case 'xlsx':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#15803d; border-radius:3px; font-family:system-ui; font-size:9px; font-weight:800; color:#fff;">X</span>`;
      case 'pptx':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#c2410c; border-radius:3px; font-family:system-ui; font-size:9px; font-weight:800; color:#fff;">P</span>`;
      case 'pdf':
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#dc2626; border-radius:3px; font-family:system-ui; font-size:7px; font-weight:800; color:#fff;">PDF</span>`;
      default:
        return `<span style="display:inline-flex; align-items:center; justify-content:center; width:17px; height:17px; background:#475569; border-radius:3px; font-size:10px; color:#fff;">📄</span>`;
    }
  },

  getKindLabel(ext, isFolder) {
    if (isFolder) return 'Folder';
    switch (ext) {
      case 'bin':
      case 'unix':
      case 'exe': return 'Unix Executable File';
      case 'mod': return 'Document';
      case 'go': return 'Go source code';
      case 'md': return 'Markdown';
      case 'png': return 'PNG Image';
      case 'jpg':
      case 'jpeg': return 'JPEG Image';
      case 'bmp': return 'BMP Bitmap Image';
      case 'webp': return 'WebP Image';
      case 'gif': return 'GIF Image';
      case 'svg': return 'SVG Vector Image';
      case 'swift': return 'Swift Source';
      case 'sh': return 'Terminal scripts';
      case 'docx': return 'Word Document';
      case 'xlsx': return 'Excel Spreadsheet';
      case 'pptx': return 'PowerPoint Presentation';
      case 'pdf': return 'PDF Document';
      default: return 'Document';
    }
  },

  formatSize(bytes) {
    const b = parseInt(bytes, 10);
    if (!b || isNaN(b) || b <= 0) return '--';
    if (b < 1024) return `${b} bytes`;
    if (b < 1024 * 1024) return `${Math.round(b / 1024)} KB`;
    const mb = (b / (1024 * 1024)).toFixed(1).replace('.', ',');
    return `${mb} MB`;
  },

  formatDate(ts) {
    if (!ts) return 'Today at 18.05';
    const num = parseInt(ts, 10);
    const d = new Date(num > 1e11 ? num : num * 1000);
    const now = new Date();

    const isToday = d.toDateString() === now.toDateString();
    const yesterday = new Date(now);
    yesterday.setDate(yesterday.getDate() - 1);
    const isYesterday = d.toDateString() === yesterday.toDateString();

    const hours = String(d.getHours()).padStart(2, '0');
    const mins = String(d.getMinutes()).padStart(2, '0');
    const timeStr = `${hours}.${mins}`;

    if (isToday) {
      return `Today at ${timeStr}`;
    }
    if (isYesterday) {
      return `Yesterday at ${timeStr}`;
    }
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    return `${d.getDate()} ${months[d.getMonth()]} ${d.getFullYear()} at ${timeStr}`;
  }
};