/**
 * DeckCraft - Sovereign Presentation Studio
 * Full-fidelity web adaptation of storytold/deckcraft (Pure Rust & Engine-First Architecture)
 * Features:
 * - Office-grade Ribbon: File, Home, Insert, Design, Transitions, Slide Show
 * - Command-driven Undo/Redo stack (Ctrl+Z / Ctrl+Y)
 * - 3-Panel Studio: Live Mini Slide Sorter, 16:9 Interactive Drag & Resize Canvas, Properties Inspector
 * - Slide Layouts (Blank, Title Slide, Title & Content, Two Column, Section Header)
 * - Elements: Title, Text Box, Badge, Shapes (Rectangle, Circle, Arrow), Card, Table, SVG Chart, Image
 * - Design Theme Presets, Transitions (Fade, Slide, Zoom, Flip), & Speaker Notes
 * - Fullscreen Presenter Mode with Laser Pointer, Live Timer, Speaker Notes toggle, & Keyboard Nav
 * - Native Finder integration (Open, Save, Save As .pptx/.deckcraft, Print/PDF) & 100% Bilingual (ID/EN)
 */

window.DokuDeckCraft = {
  container: null,
  currentFile: null,
  activeTab: 'home',
  slides: [],
  activeSlideIdx: 0,
  selectedElementIdx: null,
  isPresenterMode: false,
  presenterNotesOpen: false,
  laserActive: false,
  presenterStartTime: null,
  presenterTimerInterval: null,
  showNotes: true,
  showInspector: true,
  zoomLevel: 100,
  isDirty: false,
  undoStack: [],
  redoStack: [],
  _keyHandler: null,
  _availableDocs: [],

  themes: [
    { id: 'clean', name: 'Clean Light', bg: '#ffffff', titleColor: '#0f172a', textColor: '#475569', accent: '#d24726' },
    { id: 'midnight', name: 'Midnight Slate', bg: 'linear-gradient(135deg, #1e293b 0%, #0f172a 100%)', titleColor: '#f8fafc', textColor: '#94a3b8', accent: '#38bdf8' },
    { id: 'tahoe', name: 'macOS Tahoe', bg: 'linear-gradient(135deg, #0ea5e9 0%, #6366f1 55%, #a855f7 100%)', titleColor: '#ffffff', textColor: '#e2e8f0', accent: '#fbbf24' },
    { id: 'corporate', name: 'Corporate Navy', bg: 'linear-gradient(135deg, #0f172a 0%, #1e3a8a 100%)', titleColor: '#ffffff', textColor: '#cbd5e1', accent: '#3b82f6' },
    { id: 'emerald', name: 'Emerald Cloud', bg: 'linear-gradient(135deg, #064e3b 0%, #047857 100%)', titleColor: '#ecfdf5', textColor: '#a7f3d0', accent: '#34d399' },
    { id: 'sunset', name: 'Warm Sunset', bg: 'linear-gradient(135deg, #fff7ed 0%, #ffedd5 100%)', titleColor: '#7c2d12', textColor: '#9a3412', accent: '#ea580c' }
  ],

  getAuthHeaders() {
    const headers = {};
    const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
    if (token) headers['Authorization'] = `Bearer ${token}`;
    const curUser = (window.DokuHub && DokuHub.currentUser) || null;
    if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;
    return headers;
  },

  async init(containerEl, file = null) {
    this.container = containerEl;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    this.currentFile = file || {
      id: 'new_deck',
      name: isId ? 'Presentasi Baru.pptx' : 'Untitled Presentation.pptx'
    };
    this.activeTab = 'home';
    this.activeSlideIdx = 0;
    this.selectedElementIdx = null;
    this.zoomLevel = 100;
    this.isDirty = false;
    this.undoStack = [];
    this.redoStack = [];

    await this.loadDeck();
    this.render();
    this.updateWindowTitle();
    this.bindShortcuts();
  },

  bindShortcuts() {
    if (this._shortcutHandler) {
      window.removeEventListener('keydown', this._shortcutHandler);
    }
    this._shortcutHandler = (e) => {
      if (!window.DokuHub || DokuHub.activeAppId !== 'deckcraft' || this.isPresenterMode) return;
      const inEditable = e.target && (e.target.isContentEditable || e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA');

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        if (e.shiftKey) this.redo();
        else this.undo();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
        e.preventDefault();
        this.redo();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        this.saveDeck();
      } else if (e.key === 'Delete' && !inEditable && this.selectedElementIdx !== null) {
        e.preventDefault();
        this.deleteSelectedElement();
      } else if (e.key === 'F5') {
        e.preventDefault();
        this.startPresenterMode(0);
      }
    };
    window.addEventListener('keydown', this._shortcutHandler);
  },

  updateWindowTitle() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const docName = (this.currentFile && this.currentFile.name)
      ? this.currentFile.name
      : (isId ? 'Presentasi Baru.pptx' : 'Untitled Presentation.pptx');

    const win = document.getElementById('window-deckcraft');
    if (win) {
      const titleEl = win.querySelector('.window-title');
      if (titleEl) {
        titleEl.innerHTML = `📽️ DeckCraft — ${docName}${this.isDirty ? ' <span style="color:#ef4444; font-size:10px;">●</span>' : ''}`;
      }
    }
  },

  /* ------------------------------------------------------------------------
     Command History (Undo / Redo Snapshot Engine)
     ------------------------------------------------------------------------ */
  pushHistory() {
    const snapshot = JSON.stringify({
      slides: this.slides,
      activeSlideIdx: this.activeSlideIdx
    });
    this.undoStack.push(snapshot);
    if (this.undoStack.length > 40) this.undoStack.shift();
    this.redoStack = [];
    this.isDirty = true;
    this.updateWindowTitle();
  },

  undo() {
    if (this.undoStack.length === 0) return;
    this.redoStack.push(JSON.stringify({ slides: this.slides, activeSlideIdx: this.activeSlideIdx }));
    const prev = JSON.parse(this.undoStack.pop());
    this.slides = prev.slides;
    this.activeSlideIdx = Math.min(prev.activeSlideIdx, this.slides.length - 1);
    this.selectedElementIdx = null;
    this.render();
  },

  redo() {
    if (this.redoStack.length === 0) return;
    this.undoStack.push(JSON.stringify({ slides: this.slides, activeSlideIdx: this.activeSlideIdx }));
    const next = JSON.parse(this.redoStack.pop());
    this.slides = next.slides;
    this.activeSlideIdx = Math.min(next.activeSlideIdx, this.slides.length - 1);
    this.selectedElementIdx = null;
    this.render();
  },

  /* ------------------------------------------------------------------------
     Data Loading & Blank Slide Initialization
     ------------------------------------------------------------------------ */
  async loadDeck() {
    if (!this.currentFile || this.currentFile.id === 'new_deck') {
      this.initBlankDeck();
      return;
    }

    try {
      const res = await fetch(`/api/files/${this.currentFile.id}/content`, {
        headers: this.getAuthHeaders()
      });
      if (res.ok) {
        const text = await res.text();
        if (text && text.trim().startsWith('{')) {
          const json = JSON.parse(text);
          if (json.slides && Array.isArray(json.slides) && json.slides.length > 0) {
            this.slides = json.slides;
            this.activeSlideIdx = 0;
            return;
          }
        }
      }
    } catch (e) {
      console.warn('DeckCraft load fallback:', e);
    }

    this.initBlankDeck();
  },

  initBlankDeck() {
    this.slides = [
      {
        id: `slide_${Date.now()}`,
        background: '#ffffff',
        transition: 'fade',
        duration: 0.4,
        notes: '',
        elements: []
      }
    ];
    this.activeSlideIdx = 0;
    this.selectedElementIdx = null;
  },

  /* ------------------------------------------------------------------------
     Main Studio Render
     ------------------------------------------------------------------------ */
  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curSlide = this.slides[this.activeSlideIdx] || this.slides[0];
    if (!curSlide) return;

    const scale = this.zoomLevel / 100;

    this.container.innerHTML = `
      <div class="deck-studio-layout">
        <!-- 1. Office-Grade Ribbon Bar -->
        <div class="wordcraft-ribbon-bar" style="background:var(--window-header-bg); border-bottom:1px solid var(--window-border); flex-shrink:0;">
          <div class="wordcraft-tabs-header" style="display:flex; align-items:center; height:34px; padding:0 8px; border-bottom:1px solid var(--window-border); gap:2px;">
            <button class="wordcraft-tab-btn tab-file" onclick="DokuDeckCraft.toggleFileMenu(event)" style="background:#d24726 !important;">
              ${isId ? 'Berkas' : 'File'} ▾
            </button>
            ${['home', 'insert', 'design', 'transitions', 'slideshow'].map(tab => {
              const labels = {
                home: isId ? 'Beranda' : 'Home',
                insert: isId ? 'Sisipkan' : 'Insert',
                design: isId ? 'Desain' : 'Design',
                transitions: isId ? 'Transisi' : 'Transitions',
                slideshow: isId ? 'Tayangan Slide' : 'Slide Show'
              };
              const active = this.activeTab === tab;
              return `
                <button class="wordcraft-tab-btn ${active ? 'active' : ''}" onclick="DokuDeckCraft.switchTab('${tab}')"
                        style="${active ? 'color:#d24726; border-bottom-color:#d24726;' : ''}">
                  ${labels[tab]}
                </button>
              `;
            }).join('')}

            <div style="flex:1;"></div>

            <!-- Quick Action Bar -->
            <div style="display:flex; align-items:center; gap:6px; margin-right:6px;">
              <span style="font-size:11px; color:${this.isDirty ? '#f59e0b' : '#10b981'}; font-weight:600; margin-right:4px;">
                ● ${this.isDirty ? (isId ? 'Belum Disimpan' : 'Unsaved') : (isId ? 'Tersimpan' : 'Saved')}
              </span>
              <button class="mac-btn" onclick="DokuDeckCraft.openFromFileManagerModal()" style="height:24px; padding:0 8px; font-size:11px;">
                📂 ${isId ? 'Buka...' : 'Open...'}
              </button>
              <button class="mac-btn primary" onclick="DokuDeckCraft.saveDeck()" style="height:24px; padding:0 10px; font-size:11px; background:#d24726; border-color:#d24726;">
                💾 ${isId ? 'Simpan' : 'Save'}
              </button>
              <button class="mac-btn primary" onclick="DokuDeckCraft.startPresenterMode(DokuDeckCraft.activeSlideIdx)" style="height:24px; padding:0 10px; font-size:11px; background:#10b981; border-color:#10b981;">
                ▶ ${isId ? 'Presentasi' : 'Present'}
              </button>
            </div>
          </div>

          <!-- Active Ribbon Tab Controls -->
          <div class="wordcraft-ribbon-panel" style="padding:6px 12px 4px; min-height:76px; display:flex; align-items:stretch; gap:8px; overflow-x:auto;">
            ${this.renderRibbonTab(isId, curSlide)}
          </div>
        </div>

        <!-- 2. 3-Panel Studio Workspace -->
        <div class="deck-workspace">
          <!-- Left: Slide Sorter Thumbnails -->
          <div class="deck-slides-pane">
            <div style="display:flex; justify-content:space-between; align-items:center; padding:0 4px;">
              <span style="font-size:11px; font-weight:700; color:var(--text-muted);">SLIDES (${this.slides.length})</span>
              <button class="mac-btn primary" style="padding:2px 8px; font-size:11px; background:#d24726; border-color:#d24726;" onclick="DokuDeckCraft.addSlide('blank')">
                + ${isId ? 'Slide' : 'Slide'}
              </button>
            </div>

            ${this.slides.map((s, idx) => `
              <div class="deck-thumb-card ${idx === this.activeSlideIdx ? 'active' : ''}" onclick="DokuDeckCraft.selectSlide(${idx})">
                <span style="font-size:11px; font-weight:700; color:var(--text-muted); width:16px; text-align:right; padding-top:2px;">${idx + 1}</span>
                <div class="deck-thumb-preview" style="background:${s.background || '#ffffff'};">
                  <div style="width:800px; height:450px; transform:scale(0.19); transform-origin:top left; pointer-events:none; position:relative;">
                    ${(s.elements || []).map((el, elIdx) => this.renderElementHtml(el, elIdx, true)).join('')}
                  </div>
                </div>
              </div>
            `).join('')}
          </div>

          <!-- Center: Interactive 16:9 Canvas + Speaker Notes -->
          <div class="deck-canvas-stage">
            <div class="deck-canvas-viewport" onclick="if(event.target===this) DokuDeckCraft.selectElement(null)">
              <div class="deck-slide-board" id="deck-slide-board"
                   style="background:${curSlide.background || '#ffffff'}; transform:scale(${scale});"
                   onclick="if(event.target===this) DokuDeckCraft.selectElement(null)">
                ${(curSlide.elements || []).length === 0 ? `
                  <div style="position:absolute; inset:0; display:flex; align-items:center; justify-content:center; flex-direction:column; color:rgba(125,125,125,0.45); pointer-events:none; text-align:center; padding:24px;">
                    <div style="font-size:32px; margin-bottom:6px; opacity:0.6;">📽️</div>
                    <div style="font-size:14px; font-weight:600;">${isId ? 'Slide Kosong (Blank)' : 'Blank Slide'}</div>
                    <div style="font-size:12px; margin-top:4px;">${isId ? 'Gunakan tab Sisipkan atau pilih Tata Letak Slide di Ribbon atas.' : 'Use the Insert tab or choose a Slide Layout from the Ribbon above.'}</div>
                  </div>
                ` : (curSlide.elements || []).map((el, elIdx) => this.renderElementHtml(el, elIdx, false)).join('')}
              </div>
            </div>

            ${this.showNotes ? `
              <div class="deck-notes-bar">
                <div style="font-size:10.5px; font-weight:700; color:var(--text-muted); margin-bottom:4px; display:flex; justify-content:space-between;">
                  <span>📝 ${isId ? 'Catatan Pembicara (Speaker Notes)' : 'Speaker Notes'} — Slide ${this.activeSlideIdx + 1}</span>
                  <span style="cursor:pointer;" onclick="DokuDeckCraft.showNotes=false; DokuDeckCraft.render();">✕</span>
                </div>
                <textarea placeholder="${isId ? 'Ketik catatan presentasi Anda untuk slide ini...' : 'Click to add speaker notes for this slide...'}"
                          oninput="DokuDeckCraft.updateSlideNotes(this.value)"
                          style="flex:1; border:none; outline:none; resize:none; background:transparent; color:var(--text-main); font-size:12px; font-family:inherit; line-height:1.45;">${curSlide.notes || ''}</textarea>
              </div>
            ` : ''}
          </div>

          <!-- Right: Properties Inspector Panel -->
          ${this.showInspector ? `
            <div class="deck-inspector-pane">
              ${this.renderInspector(isId, curSlide)}
            </div>
          ` : ''}
        </div>

        <!-- 3. Bottom Status Bar -->
        <div class="md-statusbar">
          <div style="display:flex; align-items:center; gap:14px;">
            <span style="font-weight:700; color:#d24726;">SLIDE ${this.activeSlideIdx + 1} / ${this.slides.length}</span>
            <span>${(curSlide.elements || []).length} ${isId ? 'Elemen' : 'Elements'}</span>
            <span>${isId ? 'Transisi' : 'Transition'}: <strong>${(curSlide.transition || 'none').toUpperCase()}</strong></span>
          </div>
          <div style="display:flex; align-items:center; gap:8px;">
            <button class="ribbon-btn" style="height:18px; padding:0 6px; font-size:10.5px;" onclick="DokuDeckCraft.showNotes=!DokuDeckCraft.showNotes; DokuDeckCraft.render();">
              📝 ${isId ? 'Catatan' : 'Notes'}
            </button>
            <button class="ribbon-btn" style="height:18px; padding:0 6px; font-size:10.5px;" onclick="DokuDeckCraft.showInspector=!DokuDeckCraft.showInspector; DokuDeckCraft.render();">
              🎛️ ${isId ? 'Inspektor' : 'Inspector'}
            </button>
            <button class="ribbon-btn" style="height:18px; width:18px; padding:0; font-size:10px;" onclick="DokuDeckCraft.setZoom(DokuDeckCraft.zoomLevel - 10)">-</button>
            <span style="font-weight:600; min-width:34px; text-align:center;">${this.zoomLevel}%</span>
            <button class="ribbon-btn" style="height:18px; width:18px; padding:0; font-size:10px;" onclick="DokuDeckCraft.setZoom(DokuDeckCraft.zoomLevel + 10)">+</button>
          </div>
        </div>
      </div>

      <!-- Fullscreen Presenter Mode Overlay -->
      <div id="deck-presenter-overlay" style="display:none; position:fixed; inset:0; background:#05070c; z-index:9999999; flex-direction:column; align-items:center; justify-content:center; color:#fff;">
        <div id="presenter-slide-wrap" style="position:relative; width:90vw; max-width:1280px; aspect-ratio:16/9; border-radius:10px; overflow:hidden; box-shadow:0 0 60px rgba(0,0,0,0.7);">
          <div id="presenter-slide-box" style="width:100%; height:100%; position:relative;"></div>
          <div id="presenter-laser-dot" style="display:none; position:absolute; width:14px; height:14px; border-radius:50%; background:radial-gradient(circle, #ff3b30 0%, rgba(255,59,48,0.6) 60%, transparent 100%); box-shadow:0 0 14px #ff3b30; pointer-events:none; transform:translate(-50%,-50%); z-index:99;"></div>
        </div>

        <div id="presenter-notes-drawer" style="display:none; margin-top:12px; width:90vw; max-width:960px; background:rgba(30,32,40,0.92); border:1px solid rgba(255,255,255,0.16); border-radius:12px; padding:12px 16px; font-size:13px; color:#e2e8f0; max-height:110px; overflow-y:auto;"></div>

        <div style="position:fixed; bottom:20px; display:flex; gap:12px; align-items:center; background:rgba(20,22,28,0.88); backdrop-filter:blur(20px); padding:8px 18px; border-radius:30px; border:1px solid rgba(255,255,255,0.16); color:#fff; font-size:12.5px;">
          <span id="presenter-timer" style="font-family:monospace; font-weight:700; color:#34d399;">00:00</span>
          <button class="mac-btn" onclick="DokuDeckCraft.prevSlide()">◀</button>
          <span id="presenter-slide-counter" style="font-weight:700;">1 / 1</span>
          <button class="mac-btn" onclick="DokuDeckCraft.nextSlide()">▶</button>
          <button class="mac-btn" id="btn-presenter-laser" onclick="DokuDeckCraft.toggleLaser()">🔴 Laser</button>
          <button class="mac-btn" onclick="DokuDeckCraft.togglePresenterNotes()">📝 ${isId ? 'Catatan' : 'Notes'}</button>
          <button class="mac-btn danger" style="background:#ef4444; color:#fff; border:none;" onclick="DokuDeckCraft.exitPresenterMode()">✕ ${isId ? 'Keluar (Esc)' : 'Exit (Esc)'}</button>
        </div>
      </div>
    `;

    this.attachCanvasHandlers();
  },

  renderRibbonTab(isId, curSlide) {
    const selEl = (this.selectedElementIdx !== null && curSlide.elements[this.selectedElementIdx])
      ? curSlide.elements[this.selectedElementIdx]
      : null;

    if (this.activeTab === 'home') {
      return `
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuDeckCraft.addSlide('blank')" style="font-weight:600; color:#d24726;">➕ ${isId ? 'Slide Kosong' : 'Blank Slide'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.duplicateSlide()">📄 ${isId ? 'Duplikat' : 'Duplicate'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.deleteSlide()" style="color:#ef4444;">🗑️ ${isId ? 'Hapus' : 'Delete'}</button>
          </div>
          <div class="ribbon-row">
            <select class="ribbon-select" onchange="if(this.value){ DokuDeckCraft.applySlideLayout(this.value); this.value=''; }" style="width:170px;">
              <option value="">📐 ${isId ? 'Terapkan Tata Letak...' : 'Apply Slide Layout...'}</option>
              <option value="title">${isId ? 'Slide Judul (Title Slide)' : 'Title Slide'}</option>
              <option value="content">${isId ? 'Judul & Konten' : 'Title & Content'}</option>
              <option value="two_col">${isId ? 'Dua Kolom (Two Column)' : 'Two Column'}</option>
              <option value="section">${isId ? 'Header Bagian (Section)' : 'Section Header'}</option>
              <option value="blank">${isId ? 'Kosong (Blank)' : 'Blank'}</option>
            </select>
            <button class="ribbon-btn" onclick="DokuDeckCraft.moveSlide(-1)" title="${isId ? 'Geser Slide ke Atas' : 'Move Slide Up'}">⬆️</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.moveSlide(1)" title="${isId ? 'Geser Slide ke Bawah' : 'Move Slide Down'}">⬇️</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Slide & Tata Letak' : 'Slides & Layout'}</div>
        </div>

        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuDeckCraft.undo()" title="Undo (Ctrl+Z)">↩️ Undo</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.redo()" title="Redo (Ctrl+Y)">↪️ Redo</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuDeckCraft.duplicateSelectedElement()">📑 ${isId ? 'Duplikat Objek' : 'Dup Object'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.deleteSelectedElement()" style="color:#ef4444;">✕ ${isId ? 'Hapus Objek' : 'Del Object'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Riwayat & Objek' : 'History & Objects'}</div>
        </div>

        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn ${selEl && selEl.bold ? 'active' : ''}" onclick="DokuDeckCraft.toggleSelectedProp('bold')"><strong>B</strong></button>
            <button class="ribbon-btn ${selEl && selEl.italic ? 'active' : ''}" onclick="DokuDeckCraft.toggleSelectedProp('italic')"><em>I</em></button>
            <button class="ribbon-btn ${selEl && selEl.underline ? 'active' : ''}" onclick="DokuDeckCraft.toggleSelectedProp('underline')"><u>U</u></button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.setSelectedProp('align', 'left')">⇤</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.setSelectedProp('align', 'center')">↔</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.setSelectedProp('align', 'right')">⇥</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuDeckCraft.bringForward()">⬆️ ${isId ? 'Ke Depan' : 'Front'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.sendBackward()">⬇️ ${isId ? 'Ke Belakang' : 'Back'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Format & Susunan' : 'Format & Arrange'}</div>
        </div>
      `;
    }

    if (this.activeTab === 'insert') {
      return `
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('title')">🔠 ${isId ? 'Judul' : 'Title'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('text')">📝 ${isId ? 'Kotak Teks' : 'Text Box'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('badge')">🏷️ Badge</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('card')">🃏 ${isId ? 'Kartu Info' : 'Info Card'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Teks & Kartu' : 'Text & Cards'}</div>
        </div>

        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('rect')">▭ ${isId ? 'Persegi' : 'Rectangle'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('circle')">◯ ${isId ? 'Lingkaran' : 'Circle'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('arrow')">➔ ${isId ? 'Panah' : 'Arrow'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Bentuk (Shapes)' : 'Shapes'}</div>
        </div>

        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('image')">🖼️ ${isId ? 'Gambar (URL)' : 'Image (URL)'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('table')">⊞ ${isId ? 'Tabel 3×3' : 'Table 3×3'}</button>
            <button class="ribbon-btn" onclick="DokuDeckCraft.addElement('chart')">📊 ${isId ? 'Grafik SVG' : 'SVG Chart'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Media & Data' : 'Media & Data'}</div>
        </div>
      `;
    }

    if (this.activeTab === 'design') {
      return `
        <div class="ribbon-group">
          <div class="ribbon-row" style="gap:6px;">
            ${this.themes.map(th => `
              <button onclick="DokuDeckCraft.applyTheme('${th.id}')" title="${th.name}"
                      style="width:74px; height:44px; border-radius:8px; border:1px solid var(--window-border); background:${th.bg}; color:${th.titleColor}; font-size:10px; font-weight:700; cursor:pointer; display:flex; align-items:flex-end; justify-content:center; padding-bottom:4px; box-shadow:0 2px 6px rgba(0,0,0,0.12);">
                ${th.name.split(' ')[0]}
              </button>
            `).join('')}
          </div>
          <div class="ribbon-group-title">${isId ? 'Tema Presentasi' : 'Presentation Themes'}</div>
        </div>

        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <label class="ribbon-btn" style="cursor:pointer; position:relative;">
              🎨 ${isId ? 'Warna Latar' : 'Fill Color'}
              <input type="color" onchange="DokuDeckCraft.setSlideBackground(this.value)" style="opacity:0; position:absolute; inset:0; cursor:pointer;">
            </label>
            <button class="ribbon-btn" onclick="DokuDeckCraft.promptSlideBackground()">🌈 CSS Gradient</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuDeckCraft.applyBgToAll()">✨ ${isId ? 'Terapkan ke Semua Slide' : 'Apply to All Slides'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Latar Belakang' : 'Background'}</div>
        </div>
      `;
    }

    if (this.activeTab === 'transitions') {
      const trans = [
        { id: 'none', label: isId ? 'Tanpa Transisi' : 'None', icon: '⊘' },
        { id: 'fade', label: 'Fade', icon: '🌫️' },
        { id: 'slide', label: 'Slide', icon: '➡️' },
        { id: 'zoom', label: 'Zoom', icon: '🔍' },
        { id: 'flip', label: 'Flip 3D', icon: '🔄' }
      ];
      return `
        <div class="ribbon-group">
          <div class="ribbon-row" style="gap:6px;">
            ${trans.map(tr => `
              <button class="ribbon-btn ${(curSlide.transition || 'none') === tr.id ? 'active' : ''}"
                      onclick="DokuDeckCraft.setSlideTransition('${tr.id}')"
                      style="padding:4px 10px; ${(curSlide.transition || 'none') === tr.id ? 'background:#d24726; color:#fff;' : ''}">
                ${tr.icon} ${tr.label}
              </button>
            `).join('')}
          </div>
          <div class="ribbon-group-title">${isId ? 'Efek Transisi Slide' : 'Slide Transition Effect'}</div>
        </div>
      `;
    }

    // slideshow tab
    return `
      <div class="ribbon-group">
        <div class="ribbon-row" style="gap:8px;">
          <button class="ribbon-btn" onclick="DokuDeckCraft.startPresenterMode(0)" style="font-weight:700; color:#10b981; padding:0 12px;">
            ▶ ${isId ? 'Mulai dari Awal (F5)' : 'From Beginning (F5)'}
          </button>
          <button class="ribbon-btn" onclick="DokuDeckCraft.startPresenterMode(DokuDeckCraft.activeSlideIdx)" style="font-weight:600; padding:0 12px;">
            ⏯️ ${isId ? 'Dari Slide Saat Ini' : 'From Current Slide'}
          </button>
          <button class="ribbon-btn" onclick="DokuDeckCraft.printDeck()" style="padding:0 12px;">
            🖨️ ${isId ? 'Cetak / PDF' : 'Print / PDF'}
          </button>
        </div>
        <div class="ribbon-group-title">${isId ? 'Mulai Tayangan Slide' : 'Start Slide Show'}</div>
      </div>
    `;
  },

  renderInspector(isId, curSlide) {
    const el = (this.selectedElementIdx !== null && curSlide.elements[this.selectedElementIdx])
      ? curSlide.elements[this.selectedElementIdx]
      : null;

    if (!el) {
      return `
        <div class="deck-inspector-section">
          <div class="deck-inspector-title">📽️ ${isId ? 'Properti Slide' : 'Slide Properties'}</div>
          <div style="margin-bottom:8px; color:var(--text-muted); font-size:11.5px;">
            ${isId ? 'Pilih objek pada kanvas untuk mengedit properti ukuran, warna, dan tipografinya.' : 'Select an object on the canvas to inspect and edit its geometry, colors, and typography.'}
          </div>
          <div style="display:flex; flex-direction:column; gap:6px;">
            <div style="display:flex; justify-content:space-between;"><span>${isId ? 'Nomor Slide' : 'Slide Number'}</span><strong>#${this.activeSlideIdx + 1}</strong></div>
            <div style="display:flex; justify-content:space-between;"><span>${isId ? 'Rasio' : 'Aspect Ratio'}</span><strong>16:9 (800×450)</strong></div>
            <div style="display:flex; justify-content:space-between;"><span>${isId ? 'Transisi' : 'Transition'}</span><strong>${(curSlide.transition || 'none').toUpperCase()}</strong></div>
          </div>
        </div>
      `;
    }

    return `
      <div class="deck-inspector-section">
        <div class="deck-inspector-title">🎛️ ${isId ? 'Geometri & Posisi' : 'Geometry & Position'}</div>
        <div style="display:grid; grid-template-columns:1fr 1fr; gap:6px;">
          <label style="font-size:11px;">X
            <input type="number" value="${Math.round(el.x || 0)}" onchange="DokuDeckCraft.setSelectedProp('x', parseInt(this.value)||0)"
                   style="width:100%; padding:4px 6px; border-radius:6px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main);">
          </label>
          <label style="font-size:11px;">Y
            <input type="number" value="${Math.round(el.y || 0)}" onchange="DokuDeckCraft.setSelectedProp('y', parseInt(this.value)||0)"
                   style="width:100%; padding:4px 6px; border-radius:6px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main);">
          </label>
          <label style="font-size:11px;">W
            <input type="number" value="${Math.round(el.w || 240)}" onchange="DokuDeckCraft.setSelectedProp('w', Math.max(40, parseInt(this.value)||100))"
                   style="width:100%; padding:4px 6px; border-radius:6px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main);">
          </label>
          <label style="font-size:11px;">H
            <input type="number" value="${Math.round(el.h || 60)}" onchange="DokuDeckCraft.setSelectedProp('h', Math.max(24, parseInt(this.value)||40))"
                   style="width:100%; padding:4px 6px; border-radius:6px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main);">
          </label>
        </div>
      </div>

      <div class="deck-inspector-section">
        <div class="deck-inspector-title">🎨 ${isId ? 'Tampilan & Warna' : 'Appearance & Style'}</div>
        <div style="display:flex; flex-direction:column; gap:8px;">
          <label style="display:flex; align-items:center; justify-content:space-between; font-size:11.5px;">
            <span>${isId ? 'Ukuran Font' : 'Font Size'}</span>
            <input type="number" value="${el.size || 20}" min="10" max="120" onchange="DokuDeckCraft.setSelectedProp('size', parseInt(this.value)||18)"
                   style="width:72px; padding:4px 6px; border-radius:6px; border:1px solid var(--window-border); background:var(--sidebar-bg); color:var(--text-main);">
          </label>
          <label style="display:flex; align-items:center; justify-content:space-between; font-size:11.5px;">
            <span>${isId ? 'Warna Utama' : 'Primary Color'}</span>
            <input type="color" value="${el.color && el.color.startsWith('#') ? el.color : '#0f172a'}" onchange="DokuDeckCraft.setSelectedProp('color', this.value)" style="width:42px; height:24px; border:none; cursor:pointer;">
          </label>
          <label style="display:flex; align-items:center; justify-content:space-between; font-size:11.5px;">
            <span>${isId ? 'Warna Latar' : 'Fill / Bg'}</span>
            <input type="color" value="${el.bg && el.bg.startsWith('#') ? el.bg : '#3b82f6'}" onchange="DokuDeckCraft.setSelectedProp('bg', this.value)" style="width:42px; height:24px; border:none; cursor:pointer;">
          </label>
        </div>
      </div>

      <div style="display:flex; gap:6px;">
        <button class="mac-btn" style="flex:1; justify-content:center; font-size:11px;" onclick="DokuDeckCraft.duplicateSelectedElement()">📄 ${isId ? 'Duplikat' : 'Dup'}</button>
        <button class="mac-btn" style="flex:1; justify-content:center; font-size:11px; color:#ef4444;" onclick="DokuDeckCraft.deleteSelectedElement()">🗑️ ${isId ? 'Hapus' : 'Delete'}</button>
      </div>
    `;
  },

  renderElementHtml(el, idx, isThumb = false) {
    const isSel = !isThumb && this.selectedElementIdx === idx;
    const selClass = isSel ? 'selected' : '';
    const resizeDot = isSel ? `<div class="deck-resize-dot" data-resize-idx="${idx}"></div>` : '';
    const editable = !isThumb ? 'contenteditable="true"' : '';
    const textStyle = `font-weight:${el.bold ? '800' : (el.type === 'title' ? '800' : '400')}; font-style:${el.italic ? 'italic' : 'normal'}; text-decoration:${el.underline ? 'underline' : 'none'}; text-align:${el.align || 'left'};`;

    if (el.type === 'title' || el.type === 'heading' || el.type === 'subtitle' || el.type === 'text') {
      return `
        <div class="deck-el-item ${selClass}" data-idx="${idx}"
             style="left:${el.x || 60}px; top:${el.y || 60}px; width:${el.w || 640}px; min-height:${el.h || 40}px; padding:4px 6px; font-size:${el.size || (el.type === 'title' ? 42 : 20)}px; color:${el.color || '#0f172a'}; ${textStyle} line-height:1.3;">
          <div class="deck-editable" ${editable} style="outline:none; width:100%; height:100%;">${el.text || ''}</div>
          ${resizeDot}
        </div>
      `;
    }

    if (el.type === 'badge') {
      return `
        <div class="deck-el-item ${selClass}" data-idx="${idx}"
             style="left:${el.x || 60}px; top:${el.y || 40}px; padding:6px 16px; border-radius:20px; background:${el.bg || 'rgba(16,185,129,0.16)'}; border:1px solid ${el.color || '#10b981'}; color:${el.color || '#10b981'}; font-size:${el.size || 13}px; font-weight:700;">
          <div class="deck-editable" ${editable} style="outline:none;">${el.text || 'BADGE'}</div>
          ${resizeDot}
        </div>
      `;
    }

    if (el.type === 'rect' || el.type === 'circle') {
      const radius = el.type === 'circle' ? '50%' : '12px';
      return `
        <div class="deck-el-item ${selClass}" data-idx="${idx}"
             style="left:${el.x || 120}px; top:${el.y || 120}px; width:${el.w || 160}px; height:${el.h || 120}px; border-radius:${radius}; background:${el.bg || '#3b82f6'}; color:${el.color || '#ffffff'}; display:flex; align-items:center; justify-content:center; padding:10px; font-size:${el.size || 16}px; font-weight:600; box-shadow:0 6px 18px rgba(0,0,0,0.15);">
          <div class="deck-editable" ${editable} style="outline:none; text-align:center; width:100%;">${el.text || ''}</div>
          ${resizeDot}
        </div>
      `;
    }

    if (el.type === 'arrow') {
      return `
        <div class="deck-el-item ${selClass}" data-idx="${idx}"
             style="left:${el.x || 200}px; top:${el.y || 180}px; width:${el.w || 160}px; height:${el.h || 48}px; display:flex; align-items:center;">
          <svg width="100%" height="100%" viewBox="0 0 160 48" preserveAspectRatio="none">
            <line x1="6" y1="24" x2="132" y2="24" stroke="${el.color || '#d24726'}" stroke-width="6" stroke-linecap="round"/>
            <polygon points="126,10 154,24 126,38" fill="${el.color || '#d24726'}"/>
          </svg>
          ${resizeDot}
        </div>
      `;
    }

    if (el.type === 'card') {
      return `
        <div class="deck-el-item ${selClass}" data-idx="${idx}"
             style="left:${el.x || 80}px; top:${el.y || 160}px; width:${el.w || 220}px; min-height:${el.h || 140}px; background:rgba(255,255,255,0.94); border-radius:12px; border-top:4px solid ${el.color || '#3b82f6'}; box-shadow:0 8px 22px rgba(0,0,0,0.12); padding:14px;">
          <h4 class="deck-card-title" ${editable} style="font-size:17px; font-weight:700; color:#111827; margin:0 0 6px; outline:none;">${el.title || 'Card Title'}</h4>
          <p class="deck-card-desc" ${editable} style="font-size:13px; color:#4b5563; line-height:1.45; margin:0; outline:none;">${el.desc || 'Card description'}</p>
          ${resizeDot}
        </div>
      `;
    }

    if (el.type === 'image') {
      return `
        <div class="deck-el-item ${selClass}" data-idx="${idx}"
             style="left:${el.x || 140}px; top:${el.y || 80}px; width:${el.w || 320}px; height:${el.h || 200}px; border-radius:10px; overflow:hidden; background:#e2e8f0;">
          <img src="${el.src || ''}" alt="Slide Image" style="width:100%; height:100%; object-fit:cover; pointer-events:none;">
          ${resizeDot}
        </div>
      `;
    }

    if (el.type === 'table') {
      const rows = el.rows || [['Header 1', 'Header 2', 'Header 3'], ['Data A1', 'Data B1', 'Data C1'], ['Data A2', 'Data B2', 'Data C2']];
      return `
        <div class="deck-el-item ${selClass}" data-idx="${idx}"
             style="left:${el.x || 80}px; top:${el.y || 120}px; width:${el.w || 560}px; background:#ffffff; border-radius:8px; overflow:hidden; box-shadow:0 6px 18px rgba(0,0,0,0.12);">
          <table style="width:100%; border-collapse:collapse; font-size:13px; color:#1e293b;">
            ${rows.map((r, rIdx) => `
              <tr>
                ${r.map(c => rIdx === 0
                  ? `<th ${editable} style="background:${el.color || '#d24726'}; color:#fff; padding:8px 10px; text-align:left; border:1px solid rgba(0,0,0,0.08); outline:none;">${c}</th>`
                  : `<td ${editable} style="padding:8px 10px; border:1px solid #e2e8f0; outline:none;">${c}</td>`
                ).join('')}
              </tr>
            `).join('')}
          </table>
          ${resizeDot}
        </div>
      `;
    }

    if (el.type === 'chart') {
      const vals = el.values || [65, 88, 52, 95];
      const labels = el.labels || ['Q1', 'Q2', 'Q3', 'Q4'];
      const maxVal = Math.max(...vals, 100);
      return `
        <div class="deck-el-item ${selClass}" data-idx="${idx}"
             style="left:${el.x || 120}px; top:${el.y || 110}px; width:${el.w || 420}px; height:${el.h || 240}px; background:rgba(255,255,255,0.95); border-radius:12px; padding:14px; box-shadow:0 8px 22px rgba(0,0,0,0.12); display:flex; flex-direction:column;">
          <div style="font-size:13px; font-weight:700; color:#1e293b; margin-bottom:8px;">${el.title || 'Performance Overview'}</div>
          <svg viewBox="0 0 360 150" style="flex:1; width:100%;">
            ${vals.map((v, i) => {
              const barH = Math.round((v / maxVal) * 115);
              const x = 25 + i * 82;
              const y = 125 - barH;
              return `
                <rect x="${x}" y="${y}" width="48" height="${barH}" rx="6" fill="${el.color || '#d24726'}" opacity="${0.75 + i * 0.06}"/>
                <text x="${x + 24}" y="${y - 5}" text-anchor="middle" font-size="11" font-weight="700" fill="#334155">${v}%</text>
                <text x="${x + 24}" y="143" text-anchor="middle" font-size="11" font-weight="600" fill="#64748b">${labels[i] || ''}</text>
              `;
            }).join('')}
          </svg>
          ${resizeDot}
        </div>
      `;
    }

    return '';
  },

  /* ------------------------------------------------------------------------
     Canvas Drag, Resize & Direct Text Editing
     ------------------------------------------------------------------------ */
  attachCanvasHandlers() {
    const board = document.getElementById('deck-slide-board');
    if (!board) return;
    const scale = this.zoomLevel / 100;

    board.querySelectorAll('.deck-el-item').forEach(elDom => {
      const idx = parseInt(elDom.dataset.idx);

      elDom.addEventListener('mousedown', (e) => {
        if (e.target.classList.contains('deck-resize-dot')) return;
        e.stopPropagation();

        if (this.selectedElementIdx !== idx) {
          this.selectElement(idx);
          return;
        }
        if (e.target.isContentEditable && document.activeElement === e.target) return;

        let isDragging = true;
        const startX = e.clientX;
        const startY = e.clientY;
        const origX = elDom.offsetLeft;
        const origY = elDom.offsetTop;

        const onMove = (ev) => {
          if (!isDragging) return;
          const dx = (ev.clientX - startX) / scale;
          const dy = (ev.clientY - startY) / scale;
          elDom.style.left = `${Math.round(origX + dx)}px`;
          elDom.style.top = `${Math.round(origY + dy)}px`;
        };

        const onUp = () => {
          if (isDragging) {
            isDragging = false;
            const curSlide = this.slides[this.activeSlideIdx];
            if (curSlide && curSlide.elements[idx]) {
              const newX = parseInt(elDom.style.left);
              const newY = parseInt(elDom.style.top);
              if (newX !== curSlide.elements[idx].x || newY !== curSlide.elements[idx].y) {
                this.pushHistory();
                curSlide.elements[idx].x = newX;
                curSlide.elements[idx].y = newY;
                this.render();
              }
            }
          }
          window.removeEventListener('mousemove', onMove);
          window.removeEventListener('mouseup', onUp);
        };

        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
      });

      // Resize handle
      const dot = elDom.querySelector('.deck-resize-dot');
      if (dot) {
        dot.addEventListener('mousedown', (e) => {
          e.stopPropagation();
          e.preventDefault();
          const startX = e.clientX;
          const startY = e.clientY;
          const startW = elDom.offsetWidth;
          const startH = elDom.offsetHeight;

          const onMove = (ev) => {
            const dx = (ev.clientX - startX) / scale;
            const dy = (ev.clientY - startY) / scale;
            elDom.style.width = `${Math.max(50, Math.round(startW + dx))}px`;
            elDom.style.height = `${Math.max(28, Math.round(startH + dy))}px`;
          };

          const onUp = () => {
            const curSlide = this.slides[this.activeSlideIdx];
            if (curSlide && curSlide.elements[idx]) {
              this.pushHistory();
              curSlide.elements[idx].w = parseInt(elDom.style.width);
              curSlide.elements[idx].h = parseInt(elDom.style.height);
              this.render();
            }
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
          };

          window.addEventListener('mousemove', onMove);
          window.addEventListener('mouseup', onUp);
        });
      }

      // Sync edited text on blur
      elDom.querySelectorAll('[contenteditable="true"]').forEach(ed => {
        ed.addEventListener('blur', () => {
          const curSlide = this.slides[this.activeSlideIdx];
          if (!curSlide || !curSlide.elements[idx]) return;
          const item = curSlide.elements[idx];
          if (item.type === 'card') {
            const h4 = elDom.querySelector('.deck-card-title');
            const p = elDom.querySelector('.deck-card-desc');
            if (h4) item.title = h4.innerText;
            if (p) item.desc = p.innerText;
          } else if (item.type !== 'table') {
            item.text = ed.innerText;
          }
          this.isDirty = true;
          this.updateWindowTitle();
        });
      });
    });
  },

  /* ------------------------------------------------------------------------
     Ribbon & Slide Commands
     ------------------------------------------------------------------------ */
  switchTab(tab) {
    this.activeTab = tab;
    this.render();
  },

  selectSlide(idx) {
    this.activeSlideIdx = idx;
    this.selectedElementIdx = null;
    this.render();
    if (this.isPresenterMode) this.updatePresenterSlide();
  },

  selectElement(idx) {
    this.selectedElementIdx = idx;
    this.render();
  },

  nextSlide() {
    if (this.activeSlideIdx < this.slides.length - 1) {
      this.selectSlide(this.activeSlideIdx + 1);
    }
  },

  prevSlide() {
    if (this.activeSlideIdx > 0) {
      this.selectSlide(this.activeSlideIdx - 1);
    }
  },

  addSlide(layout = 'blank') {
    this.pushHistory();
    const newSlide = {
      id: `slide_${Date.now()}`,
      background: '#ffffff',
      transition: 'fade',
      duration: 0.4,
      notes: '',
      elements: []
    };
    this.slides.push(newSlide);
    this.activeSlideIdx = this.slides.length - 1;
    if (layout !== 'blank') {
      this.applySlideLayout(layout, false);
    } else {
      this.selectedElementIdx = null;
      this.render();
    }
  },

  duplicateSlide() {
    this.pushHistory();
    const copy = JSON.parse(JSON.stringify(this.slides[this.activeSlideIdx]));
    copy.id = `slide_${Date.now()}`;
    this.slides.splice(this.activeSlideIdx + 1, 0, copy);
    this.activeSlideIdx++;
    this.selectedElementIdx = null;
    this.render();
  },

  moveSlide(delta) {
    const target = this.activeSlideIdx + delta;
    if (target < 0 || target >= this.slides.length) return;
    this.pushHistory();
    const [item] = this.slides.splice(this.activeSlideIdx, 1);
    this.slides.splice(target, 0, item);
    this.activeSlideIdx = target;
    this.render();
  },

  deleteSlide() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (this.slides.length <= 1) {
      if (window.DokuIsland) DokuIsland.info(isId ? 'Minimal harus ada 1 slide.' : 'At least 1 slide is required.', 'DeckCraft');
      return;
    }
    this.pushHistory();
    this.slides.splice(this.activeSlideIdx, 1);
    this.activeSlideIdx = Math.max(0, this.activeSlideIdx - 1);
    this.selectedElementIdx = null;
    this.render();
  },

  applySlideLayout(layout, recordHistory = true) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide) return;
    if (recordHistory) this.pushHistory();

    if (layout === 'blank') {
      curSlide.elements = [];
    } else if (layout === 'title') {
      curSlide.elements = [
        { type: 'title', text: isId ? 'Judul Presentasi Utama' : 'Presentation Main Title', x: 80, y: 150, w: 640, h: 65, size: 46, color: '#0f172a', bold: true, align: 'center' },
        { type: 'subtitle', text: isId ? 'Subjudul atau nama pembicara di sini' : 'Subtitle or presenter name goes here', x: 120, y: 235, w: 560, h: 40, size: 20, color: '#64748b', align: 'center' }
      ];
    } else if (layout === 'content') {
      curSlide.elements = [
        { type: 'title', text: isId ? 'Judul Topik Pembahasan' : 'Topic Headline', x: 56, y: 40, w: 680, h: 55, size: 34, color: '#0f172a', bold: true },
        { type: 'text', text: isId ? '• Poin pembahasan pertama yang jelas dan ringkas\n• Poin kedua untuk mendukung data presentasi\n• Poin ketiga sebagai kesimpulan utama' : '• First key discussion point\n• Second supporting detail\n• Third takeaway conclusion', x: 56, y: 118, w: 680, h: 240, size: 20, color: '#334155' }
      ];
    } else if (layout === 'two_col') {
      curSlide.elements = [
        { type: 'title', text: isId ? 'Perbandingan Dua Kolom' : 'Two Column Comparison', x: 56, y: 36, w: 680, h: 50, size: 32, color: '#0f172a', bold: true },
        { type: 'card', title: isId ? 'Kolom Pertama' : 'First Column', desc: isId ? 'Uraikan poin utama atau keunggulan pertama di sini.' : 'Describe the first key pillar or insight here.', x: 56, y: 115, w: 320, h: 240, color: '#d24726' },
        { type: 'card', title: isId ? 'Kolom Kedua' : 'Second Column', desc: isId ? 'Uraikan poin pembanding atau solusi kedua di sini.' : 'Describe the second comparison or solution here.', x: 416, y: 115, w: 320, h: 240, color: '#2563eb' }
      ];
    } else if (layout === 'section') {
      curSlide.background = 'linear-gradient(135deg, #1e293b 0%, #0f172a 100%)';
      curSlide.elements = [
        { type: 'badge', text: isId ? 'BAGIAN 01' : 'SECTION 01', x: 80, y: 135, color: '#38bdf8', bg: 'rgba(56,189,248,0.16)' },
        { type: 'title', text: isId ? 'Bab Baru Presentasi' : 'New Section Header', x: 80, y: 185, w: 640, h: 64, size: 44, color: '#ffffff', bold: true }
      ];
    }

    this.selectedElementIdx = null;
    this.render();
  },

  addElement(type) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide) return;

    if (type === 'image') {
      const url = prompt(isId ? 'Masukkan URL gambar (PNG/JPG/SVG):' : 'Enter image URL (PNG/JPG/SVG):', 'https://images.unsplash.com/photo-1557683316-973673baf926?w=800');
      if (!url) return;
      this.pushHistory();
      curSlide.elements.push({ type: 'image', src: url.trim(), x: 160, y: 90, w: 340, h: 210 });
    } else {
      this.pushHistory();
      const map = {
        title: { type: 'title', text: isId ? 'Judul Baru' : 'New Title', x: 80, y: 60, w: 560, h: 56, size: 38, color: '#0f172a', bold: true },
        text: { type: 'text', text: isId ? 'Klik dua kali untuk menulis teks presentasi Anda...' : 'Double-click to edit text content...', x: 80, y: 150, w: 520, h: 70, size: 19, color: '#334155' },
        badge: { type: 'badge', text: isId ? 'LABEL BARU' : 'NEW BADGE', x: 80, y: 50, color: '#10b981', bg: 'rgba(16,185,129,0.16)', size: 12 },
        card: { type: 'card', title: isId ? 'Kartu Informasi' : 'Info Card', desc: isId ? 'Tuliskan ringkasan poin penting di dalam kartu ini.' : 'Write a concise summary inside this card.', x: 100, y: 160, w: 240, h: 150, color: '#d24726' },
        rect: { type: 'rect', text: isId ? 'Bentuk' : 'Shape', x: 160, y: 140, w: 180, h: 110, bg: '#2563eb', color: '#ffffff', size: 16 },
        circle: { type: 'circle', text: '100%', x: 220, y: 140, w: 130, h: 130, bg: '#10b981', color: '#ffffff', size: 22 },
        arrow: { type: 'arrow', x: 240, y: 190, w: 170, h: 48, color: '#d24726' },
        table: { type: 'table', x: 90, y: 120, w: 580, h: 160, color: '#d24726', rows: [['Item', 'Q1', 'Q2'], ['Growth', '45%', '78%'], ['Target', '60%', '90%']] },
        chart: { type: 'chart', title: isId ? 'Ringkasan Kinerja' : 'Performance Overview', x: 140, y: 100, w: 440, h: 240, color: '#d24726', values: [58, 76, 68, 94], labels: ['Q1', 'Q2', 'Q3', 'Q4'] }
      };
      if (map[type]) curSlide.elements.push(map[type]);
    }

    this.selectedElementIdx = curSlide.elements.length - 1;
    this.render();
  },

  setSelectedProp(key, val) {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide || this.selectedElementIdx === null || !curSlide.elements[this.selectedElementIdx]) return;
    this.pushHistory();
    curSlide.elements[this.selectedElementIdx][key] = val;
    this.render();
  },

  toggleSelectedProp(key) {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide || this.selectedElementIdx === null || !curSlide.elements[this.selectedElementIdx]) return;
    this.pushHistory();
    curSlide.elements[this.selectedElementIdx][key] = !curSlide.elements[this.selectedElementIdx][key];
    this.render();
  },

  duplicateSelectedElement() {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide || this.selectedElementIdx === null || !curSlide.elements[this.selectedElementIdx]) return;
    this.pushHistory();
    const copy = JSON.parse(JSON.stringify(curSlide.elements[this.selectedElementIdx]));
    copy.x = (copy.x || 60) + 24;
    copy.y = (copy.y || 60) + 24;
    curSlide.elements.push(copy);
    this.selectedElementIdx = curSlide.elements.length - 1;
    this.render();
  },

  deleteSelectedElement() {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide || this.selectedElementIdx === null) return;
    this.pushHistory();
    curSlide.elements.splice(this.selectedElementIdx, 1);
    this.selectedElementIdx = null;
    this.render();
  },

  bringForward() {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide || this.selectedElementIdx === null || this.selectedElementIdx >= curSlide.elements.length - 1) return;
    this.pushHistory();
    const idx = this.selectedElementIdx;
    const [el] = curSlide.elements.splice(idx, 1);
    curSlide.elements.splice(idx + 1, 0, el);
    this.selectedElementIdx = idx + 1;
    this.render();
  },

  sendBackward() {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide || this.selectedElementIdx === null || this.selectedElementIdx <= 0) return;
    this.pushHistory();
    const idx = this.selectedElementIdx;
    const [el] = curSlide.elements.splice(idx, 1);
    curSlide.elements.splice(idx - 1, 0, el);
    this.selectedElementIdx = idx - 1;
    this.render();
  },

  applyTheme(themeId) {
    const th = this.themes.find(t => t.id === themeId);
    const curSlide = this.slides[this.activeSlideIdx];
    if (!th || !curSlide) return;
    this.pushHistory();
    curSlide.background = th.bg;
    (curSlide.elements || []).forEach(el => {
      if (el.type === 'title' || el.type === 'heading') el.color = th.titleColor;
      else if (el.type === 'subtitle' || el.type === 'text') el.color = th.textColor;
    });
    this.render();
  },

  setSlideBackground(val) {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide) return;
    this.pushHistory();
    curSlide.background = val;
    this.render();
  },

  promptSlideBackground() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide) return;
    const val = prompt(isId ? 'Masukkan warna HEX atau CSS Gradient:' : 'Enter HEX color or CSS Gradient:', curSlide.background || '#ffffff');
    if (val) this.setSlideBackground(val.trim());
  },

  applyBgToAll() {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide) return;
    this.pushHistory();
    this.slides.forEach(s => { s.background = curSlide.background; });
    this.render();
  },

  setSlideTransition(trans) {
    const curSlide = this.slides[this.activeSlideIdx];
    if (!curSlide) return;
    this.pushHistory();
    curSlide.transition = trans;
    this.render();
  },

  updateSlideNotes(val) {
    const curSlide = this.slides[this.activeSlideIdx];
    if (curSlide) {
      curSlide.notes = val;
      this.isDirty = true;
      this.updateWindowTitle();
    }
  },

  setZoom(val) {
    this.zoomLevel = Math.min(150, Math.max(50, val));
    this.render();
  },

  /* ------------------------------------------------------------------------
     File Menu, Open from Finder, Save & Save As
     ------------------------------------------------------------------------ */
  toggleFileMenu(e) {
    e.stopPropagation();
    let menu = document.getElementById('deckcraft-file-menu');
    if (!menu) {
      menu = document.createElement('div');
      menu.id = 'deckcraft-file-menu';
      menu.style.cssText = 'position:fixed; z-index:999999; background:var(--window-bg); backdrop-filter:blur(30px); border:1px solid var(--window-border); border-radius:10px; box-shadow:0 16px 40px rgba(0,0,0,0.3); padding:6px; min-width:220px; display:none;';
      document.body.appendChild(menu);
      window.addEventListener('click', () => { menu.style.display = 'none'; });
    }
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const rect = e.currentTarget.getBoundingClientRect();
    menu.innerHTML = `
      <div class="finder-menu-item" onclick="DokuDeckCraft.newBlankDeck()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>✨</span> <span>${isId ? 'Presentasi Baru (Kosong)' : 'New Blank Presentation'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuDeckCraft.openFromFileManagerModal()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📂</span> <span>${isId ? 'Buka dari Finder...' : 'Open from Finder...'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuDeckCraft.saveDeck()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>💾</span> <span>${isId ? 'Simpan (Ctrl+S)' : 'Save (Ctrl+S)'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuDeckCraft.openSaveAsModal()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📑</span> <span>${isId ? 'Simpan Sebagai...' : 'Save As...'}</span>
      </div>
      <div style="height:1px; background:var(--window-border); margin:4px 0;"></div>
      <div class="finder-menu-item" onclick="DokuDeckCraft.exportPptx()" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📽️</span> <span>${isId ? 'Ekspor PowerPoint (.pptx)' : 'Export PowerPoint (.pptx)'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuDeckCraft.exportJson()" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📦</span> <span>${isId ? 'Ekspor .deckcraft (JSON)' : 'Export .deckcraft (JSON)'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuDeckCraft.printDeck()" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>🖨️</span> <span>${isId ? 'Cetak / Ekspor PDF' : 'Print / Export PDF'}</span>
      </div>
    `;
    menu.style.left = `${rect.left}px`;
    menu.style.top = `${rect.bottom + 4}px`;
    menu.style.display = 'block';
  },

  newBlankDeck() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    this.currentFile = { id: 'new_deck', name: isId ? 'Presentasi Baru.pptx' : 'Untitled Presentation.pptx' };
    this.isDirty = false;
    this.undoStack = [];
    this.redoStack = [];
    this.initBlankDeck();
    this.render();
    this.updateWindowTitle();
  },

  async saveDeck() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (!this.currentFile || this.currentFile.id === 'new_deck') {
      this.openSaveAsModal();
      return;
    }

    const payload = JSON.stringify({
      title: this.currentFile.name,
      engine: 'deckcraft',
      slides: this.slides
    });

    try {
      const res = await fetch(`/api/files/${this.currentFile.id}/content`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json', ...this.getAuthHeaders() },
        body: payload
      });
      if (res.ok) {
        this.isDirty = false;
        this.updateWindowTitle();
        this.render();
        if (window.DokuIsland) {
          DokuIsland.success(isId ? `Presentasi "${this.currentFile.name}" tersimpan!` : `Presentation "${this.currentFile.name}" saved!`, 'DeckCraft');
        }
      }
    } catch (e) {
      alert('Save error: ' + e);
    }
  },

  async openSaveAsModal() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const defaultName = (this.currentFile && this.currentFile.name) ? this.currentFile.name : (isId ? 'Presentasi.pptx' : 'Presentation.pptx');
    const name = prompt(isId ? 'Simpan presentasi ke Finder (folder Documents):' : 'Save presentation to Finder (Documents folder):', defaultName);
    if (!name || !name.trim()) return;

    const finalName = name.trim().endsWith('.pptx') ? name.trim() : `${name.trim()}.pptx`;
    const payload = JSON.stringify({ title: finalName, engine: 'deckcraft', slides: this.slides });

    try {
      const res = await fetch('/api/files', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...this.getAuthHeaders() },
        body: JSON.stringify({
          name: finalName,
          parent_id: 'fld_docs',
          is_folder: false,
          extension: 'pptx',
          content: payload
        })
      });
      if (res.ok) {
        const created = await res.json();
        this.currentFile = created;
        this.isDirty = false;
        this.updateWindowTitle();
        this.render();
        if (window.DokuFinder) DokuFinder.loadAllFiles();
        if (window.DokuIsland) {
          DokuIsland.success(isId ? `Tersimpan sebagai "${finalName}" di Finder!` : `Saved as "${finalName}" in Finder!`, 'DeckCraft');
        }
      }
    } catch (e) {
      alert('Error: ' + e);
    }
  },

  async openFromFileManagerModal() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    let modal = document.getElementById('deckcraft-open-modal');
    if (!modal) {
      modal = document.createElement('div');
      modal.id = 'deckcraft-open-modal';
      modal.style.cssText = 'position:fixed; inset:0; background:rgba(0,0,0,0.45); backdrop-filter:blur(12px); z-index:999999; display:none; align-items:center; justify-content:center;';
      document.body.appendChild(modal);
    }
    modal.style.display = 'flex';
    modal.innerHTML = `
      <div style="width:480px; max-width:92vw; background:var(--window-bg); border:1px solid var(--window-border); border-radius:16px; box-shadow:var(--window-shadow); padding:22px; color:var(--text-main);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:14px;">
          <h3 style="font-size:16px; font-weight:700; margin:0;">📂 ${isId ? 'Buka Presentasi dari Finder' : 'Open Presentation from Finder'}</h3>
          <button class="traffic-btn btn-close" onclick="document.getElementById('deckcraft-open-modal').style.display='none'"></button>
        </div>
        <div id="deckcraft-open-list" style="max-height:260px; overflow-y:auto; display:flex; flex-direction:column; gap:6px; margin-bottom:14px;">
          <div style="padding:20px; text-align:center; color:var(--text-muted); font-size:12px;">${isId ? 'Memuat berkas...' : 'Loading files...'}</div>
        </div>
        <div style="display:flex; justify-content:flex-end;">
          <button class="mac-btn" onclick="document.getElementById('deckcraft-open-modal').style.display='none'">${isId ? 'Tutup' : 'Close'}</button>
        </div>
      </div>
    `;

    try {
      const res = await fetch('/api/files?parent_id=all&category=all', { headers: this.getAuthHeaders() });
      const files = res.ok ? await res.json() : [];
      this._availableDocs = (Array.isArray(files) ? files : []).filter(f => !f.is_folder && (f.extension === 'pptx' || f.extension === 'deckcraft'));
      const listEl = document.getElementById('deckcraft-open-list');
      if (listEl) {
        listEl.innerHTML = this._availableDocs.length === 0
          ? `<div style="padding:24px; text-align:center; color:var(--text-muted); font-size:12px;">${isId ? 'Belum ada berkas presentasi (.pptx) di Finder.' : 'No presentation (.pptx) files found in Finder.'}</div>`
          : this._availableDocs.map(d => `
              <div onclick="DokuDeckCraft.openPickerDoc('${d.id}')" style="display:flex; align-items:center; justify-content:space-between; padding:9px 12px; border-radius:8px; background:var(--card-bg); border:1px solid var(--window-border); cursor:pointer;">
                <span style="font-size:13px; font-weight:600;">📽️ ${d.name}</span>
                <button class="mac-btn primary" style="font-size:11px; padding:3px 10px;">${isId ? 'Buka' : 'Open'}</button>
              </div>
            `).join('');
      }
    } catch {}
  },

  async openPickerDoc(id) {
    const doc = this._availableDocs.find(d => d.id === id);
    const modal = document.getElementById('deckcraft-open-modal');
    if (modal) modal.style.display = 'none';
    if (doc) {
      this.currentFile = doc;
      this.isDirty = false;
      await this.loadDeck();
      this.render();
      this.updateWindowTitle();
    }
  },

  async exportPptx() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const rawName = (this.currentFile && this.currentFile.name) ? this.currentFile.name : 'Presentation.pptx';
    const fileName = rawName.toLowerCase().endsWith('.pptx') ? rawName : `${rawName}.pptx`;

    try {
      const res = await fetch('/api/files/export_pptx', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...this.getAuthHeaders() },
        body: JSON.stringify({ title: fileName, engine: 'deckcraft', slides: this.slides })
      });
      if (res.ok) {
        const blob = await res.blob();
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = fileName;
        a.click();
        URL.revokeObjectURL(a.href);
        if (window.DokuIsland) {
          DokuIsland.success(isId ? `Berkas standar PowerPoint "${fileName}" berhasil diekspor!` : `Standard PowerPoint "${fileName}" exported!`, 'DeckCraft');
        }
      }
    } catch (e) {
      alert('Export PPTX error: ' + e);
    }
  },

  exportJson() {
    const name = (this.currentFile && this.currentFile.name ? this.currentFile.name : 'presentation').replace(/\.pptx$/i, '');
    const blob = new Blob([JSON.stringify({ title: name, engine: 'deckcraft', slides: this.slides }, null, 2)], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = `${name}.deckcraft.json`;
    a.click();
    URL.revokeObjectURL(a.href);
  },

  printDeck() {
    const win = window.open('', '_blank', 'width=960,height=640');
    if (!win) return;
    const slidesHtml = this.slides.map(s => `
      <div style="width:800px; height:450px; position:relative; background:${s.background || '#fff'}; margin:16px auto; page-break-after:always; border:1px solid #e2e8f0; overflow:hidden;">
        ${(s.elements || []).map((el, i) => this.renderElementHtml(el, i, true)).join('')}
      </div>
    `).join('');
    win.document.write(`<!DOCTYPE html><html><head><title>${this.currentFile?.name || 'DeckCraft'}</title><style>body{margin:0;font-family:-apple-system,sans-serif;background:#f8fafc;}.deck-el-item{position:absolute;box-sizing:border-box;}</style></head><body>${slidesHtml}</body></html>`);
    win.document.close();
    setTimeout(() => win.print(), 300);
  },

  /* ------------------------------------------------------------------------
     Fullscreen Presenter Mode (with Transitions, Laser Pointer & Timer)
     ------------------------------------------------------------------------ */
  startPresenterMode(startIdx = 0) {
    this.activeSlideIdx = Math.max(0, Math.min(startIdx, this.slides.length - 1));
    this.isPresenterMode = true;
    this.laserActive = false;
    this.presenterStartTime = Date.now();

    const overlay = document.getElementById('deck-presenter-overlay');
    if (overlay) overlay.style.display = 'flex';
    this.updatePresenterSlide();

    if (this.presenterTimerInterval) clearInterval(this.presenterTimerInterval);
    this.presenterTimerInterval = setInterval(() => {
      const secs = Math.floor((Date.now() - this.presenterStartTime) / 1000);
      const mm = String(Math.floor(secs / 60)).padStart(2, '0');
      const ss = String(secs % 60).padStart(2, '0');
      const el = document.getElementById('presenter-timer');
      if (el) el.textContent = `${mm}:${ss}`;
    }, 1000);

    const wrap = document.getElementById('presenter-slide-wrap');
    if (wrap) {
      wrap.onmousemove = (e) => {
        const dot = document.getElementById('presenter-laser-dot');
        if (!dot || !this.laserActive) return;
        const r = wrap.getBoundingClientRect();
        dot.style.left = `${e.clientX - r.left}px`;
        dot.style.top = `${e.clientY - r.top}px`;
      };
    }

    this._keyHandler = (e) => {
      if (e.key === 'ArrowRight' || e.key === ' ' || e.key === 'PageDown') {
        e.preventDefault();
        this.nextSlide();
      } else if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
        e.preventDefault();
        this.prevSlide();
      } else if (e.key === 'Escape') {
        this.exitPresenterMode();
      }
    };
    window.addEventListener('keydown', this._keyHandler);
  },

  exitPresenterMode() {
    this.isPresenterMode = false;
    this.laserActive = false;
    const overlay = document.getElementById('deck-presenter-overlay');
    if (overlay) overlay.style.display = 'none';
    if (this.presenterTimerInterval) {
      clearInterval(this.presenterTimerInterval);
      this.presenterTimerInterval = null;
    }
    if (this._keyHandler) {
      window.removeEventListener('keydown', this._keyHandler);
      this._keyHandler = null;
    }
  },

  toggleLaser() {
    this.laserActive = !this.laserActive;
    const dot = document.getElementById('presenter-laser-dot');
    const btn = document.getElementById('btn-presenter-laser');
    if (dot) dot.style.display = this.laserActive ? 'block' : 'none';
    if (btn) btn.style.background = this.laserActive ? '#ef4444' : '';
  },

  togglePresenterNotes() {
    this.presenterNotesOpen = !this.presenterNotesOpen;
    this.updatePresenterSlide();
  },

  updatePresenterSlide() {
    const box = document.getElementById('presenter-slide-box');
    const counter = document.getElementById('presenter-slide-counter');
    const notesDrawer = document.getElementById('presenter-notes-drawer');
    if (!box) return;

    const curSlide = this.slides[this.activeSlideIdx];
    const animMap = {
      fade: 'deckTransFade 0.45s ease',
      slide: 'deckTransSlide 0.45s cubic-bezier(0.16,1,0.3,1)',
      zoom: 'deckTransZoom 0.45s cubic-bezier(0.16,1,0.3,1)',
      flip: 'deckTransFlip 0.5s cubic-bezier(0.16,1,0.3,1)'
    };
    const anim = animMap[curSlide.transition] || 'none';

    const wrap = document.getElementById('presenter-slide-wrap');
    const scaleFactor = wrap ? (wrap.clientWidth / 800) : 1;

    box.style.background = curSlide.background || '#ffffff';
    box.style.animation = 'none';
    void box.offsetWidth;
    box.style.animation = anim;
    box.innerHTML = `
      <div style="width:800px; height:450px; transform:scale(${scaleFactor}); transform-origin:top left; position:relative;">
        ${(curSlide.elements || []).map((el, i) => this.renderElementHtml(el, i, true)).join('')}
      </div>
    `;

    if (counter) counter.textContent = `${this.activeSlideIdx + 1} / ${this.slides.length}`;
    if (notesDrawer) {
      notesDrawer.style.display = this.presenterNotesOpen ? 'block' : 'none';
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      notesDrawer.textContent = curSlide.notes || (isId ? '(Tidak ada catatan pembicara untuk slide ini)' : '(No speaker notes for this slide)');
    }
  }
};
