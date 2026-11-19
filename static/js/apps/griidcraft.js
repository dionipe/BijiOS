/**
 * GridCraft - Sovereign High-Performance Spreadsheet Application
 * Full-fidelity adaptation of storytold/gridcraft (Pure Rust & Web Calculation Engine)
 * Features:
 * - Office-grade Ribbon with Tab System: File, Home, Insert, Formulas, Data, View
 * - Native .xlsx / CSV / TSV file compatibility & TOON Engine persistence
 * - Real-time Formula Calculation Engine (=SUM, =AVERAGE, =MIN, =MAX, =COUNT, =IF, =CONCAT, =VLOOKUP, arithmetic)
 * - Number Formatting: Currency (Rp / $), Percent, Decimals, Date, Custom
 * - Dynamic SVG Chart Generation (Bar, Line, Pie) based on selected grid range
 * - File Manager Picker (Open from Finder) & Destination Folder Selector (Save As)
 * - Multi-sheet Tabs, Row/Col Insertion/Deletion, Sorting, Filter, Zoom, and Status Bar metrics
 */

window.DokuGriidCraft = {
  container: null,
  currentFile: null,
  activeTab: 'home',
  activeCell: 'A1',
  selectionRange: { start: 'A1', end: 'A1' },
  isSelecting: false,
  isDirty: false,
  activeSheet: 'Sheet 1',
  sheets: ['Sheet 1'],
  data: {}, // sheetName -> { coord: { value, formula, bold, italic, underline, color, bg, align, format, decimals } }
  cols: ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P'],
  maxRows: 50,
  zoomLevel: 100,
  showGridlines: true,
  showHeadings: true,
  showFormulaBar: true,
  colWidths: {
    'A': 250,
    'B': 90,
    'C': 150,
    'D': 160,
    'E': 110,
    'F': 110,
    'G': 110,
    'H': 110,
    'I': 110,
    'J': 110,
    'K': 110,
    'L': 110,
    'M': 110,
    'N': 110,
    'O': 110,
    'P': 110
  },
  rowHeights: {},
  undoStack: [],
  redoStack: [],
  clipboardCell: null,
  _availableDocs: [],

  /* ------------------------------------------------------------------------
     Initialization
     ------------------------------------------------------------------------ */
  async init(containerEl, file = null) {
    this.container = containerEl;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    this.currentFile = file || {
      id: 'new_sheet',
      name: isId ? 'Buku Kerja Baru.xlsx' : 'Untitled Spreadsheet.xlsx'
    };

    this.activeTab = 'home';
    this.activeCell = 'A1';
    this.selectionRange = { start: 'A1', end: 'A1' };
    this.zoomLevel = 100;
    this.showGridlines = true;
    this.showHeadings = true;
    this.showFormulaBar = true;
    this.isDirty = false;
    this.sheets = ['Sheet 1'];
    this.activeSheet = 'Sheet 1';
    this.data = {};

    this.render();
    this.updateWindowTitle();
    await this.loadSpreadsheet();
  },

  updateWindowTitle() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const isNew = !this.currentFile || this.currentFile.id === 'new_sheet' || this.currentFile.name === 'Buku Kerja Baru.xlsx' || this.currentFile.name === 'Untitled Spreadsheet.xlsx';
    if (isNew && this.currentFile) {
      this.currentFile.name = isId ? 'Buku Kerja Baru.xlsx' : 'Untitled Spreadsheet.xlsx';
    }
    const docName = (this.currentFile && this.currentFile.name)
      ? this.currentFile.name
      : (isId ? 'Buku Kerja Baru.xlsx' : 'Untitled Spreadsheet.xlsx');

    const win = document.getElementById('window-griidcraft');
    if (win) {
      const titleEl = win.querySelector('.window-title');
      if (titleEl) {
        titleEl.innerHTML = `📊 GridCraft — ${docName}${this.isDirty ? ' <span style="color:#ef4444; font-size:10px;">●</span>' : ''}`;
      }
    }

    const appTitleMenu = document.getElementById('active-app-name');
    if (appTitleMenu && window.DokuHub && DokuHub.activeAppId === 'griidcraft') {
      appTitleMenu.textContent = `GridCraft — ${docName}`;
    }
  },

  async loadFile(file) {
    if (!file) return;
    this.currentFile = file;
    this.isDirty = false;
    this.updateWindowTitle();
    await this.loadSpreadsheet();
    this.updateStatusBarStatus('saved');
  },

  /* ------------------------------------------------------------------------
     Data Loading & Default Spreadsheet
     ------------------------------------------------------------------------ */
  async loadSpreadsheet() {
    if (!this.currentFile || this.currentFile.id === 'new_sheet') {
      this.sheets = ['Sheet 1'];
      this.activeSheet = 'Sheet 1';
      this.data = { 'Sheet 1': {} };
      this.render();
      return;
    }

    try {
      const headers = {};
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const curUser = (window.DokuHub && DokuHub.currentUser) || null;
      if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

      const res = await fetch(`/api/files/${this.currentFile.id}/content`, { headers });
      if (res.ok) {
        const text = await res.text();
        if (text.startsWith('{')) {
          const json = JSON.parse(text);
          if (json.sheets && Array.isArray(json.sheets) && json.sheets.length > 0) {
            this.sheets = json.sheets.map(s => s.name);
            this.activeSheet = this.sheets[0];
            json.sheets.forEach(s => {
              this.data[s.name] = s.cells || {};
            });
            this.render();
            return;
          }
        } else if (text.includes(',') || text.includes('\t') || text.includes('\n')) {
          // Parse CSV or TSV raw data
          this.parseCsvContent(text);
          this.render();
          return;
        }
      }
    } catch (e) {
      console.warn("Could not load spreadsheet content, falling back to default:", e);
    }

    this.sheets = ['Sheet 1'];
    this.activeSheet = 'Sheet 1';
    this.data = { 'Sheet 1': {} };
    this.render();
  },

  parseCsvContent(csvText) {
    const lines = csvText.split(/\r?\n/).filter(l => l.trim() !== '');
    const sheetData = {};
    lines.forEach((line, rIdx) => {
      const rowNum = rIdx + 1;
      const delimiter = line.includes('\t') ? '\t' : ',';
      const cols = line.split(delimiter);
      cols.forEach((val, cIdx) => {
        if (cIdx < this.cols.length) {
          const colLetter = this.cols[cIdx];
          const coord = `${colLetter}${rowNum}`;
          const cleanVal = val.replace(/^["']|["']$/g, '').trim();
          const num = parseFloat(cleanVal);
          sheetData[coord] = {
            value: isNaN(num) || cleanVal === '' ? cleanVal : num,
            bold: rIdx === 0
          };
        }
      });
    });
    this.sheets = ['Sheet 1'];
    this.activeSheet = 'Sheet 1';
    this.data['Sheet 1'] = sheetData;
  },

  initDefaultData() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const sheet1Name = isId ? 'Laporan Keuangan' : 'Financial Report';
    const sheet2Name = isId ? 'Data Proyek' : 'Project Data';
    this.sheets = [sheet1Name, sheet2Name];
    this.activeSheet = sheet1Name;
    this.data = {
      [sheet1Name]: {
        'A1': { value: isId ? 'LAPORAN ANGGARAN & PROYEK BIJIOS' : 'BIJIOS BUDGET & PROJECT REPORT', bold: true, color: '#107c41', fontSize: 13, format: 'text' },
        'A3': { value: isId ? 'Kategori / Item' : 'Category / Item', bold: true, bg: '#f1f5f9', align: 'left', color: '#334155' },
        'B3': { value: isId ? 'Kuantitas' : 'Quantity', bold: true, bg: '#f1f5f9', align: 'center', color: '#334155' },
        'C3': { value: isId ? 'Harga Satuan (IDR)' : 'Unit Price (USD)', bold: true, bg: '#f1f5f9', align: 'right', color: '#334155' },
        'D3': { value: isId ? 'Total Biaya' : 'Total Cost', bold: true, bg: '#f1f5f9', align: 'right', color: '#334155' },

        'A4': { value: isId ? 'Server Cloud Veyla Kernel' : 'Cloud Server Cluster', align: 'left' },
        'B4': { value: 2, align: 'center' },
        'C4': { value: isId ? 12500000 : 1250, align: 'right', format: isId ? 'currency' : 'currency_usd' },
        'D4': { formula: '=B4*C4', bold: true, align: 'right', format: isId ? 'currency' : 'currency_usd' },

        'A5': { value: isId ? 'Lisensi Enterprise TOON Engine' : 'TOON Enterprise License', align: 'left' },
        'B5': { value: 5, align: 'center' },
        'C5': { value: isId ? 4500000 : 450, align: 'right', format: isId ? 'currency' : 'currency_usd' },
        'D5': { formula: '=B5*C5', bold: true, align: 'right', format: isId ? 'currency' : 'currency_usd' },

        'A6': { value: isId ? 'Penyimpanan Terisolasi NVMe' : 'NVMe Isolated Storage', align: 'left' },
        'B6': { value: 10, align: 'center' },
        'C6': { value: isId ? 750000 : 75, align: 'right', format: isId ? 'currency' : 'currency_usd' },
        'D6': { formula: '=B6*C6', bold: true, align: 'right', format: isId ? 'currency' : 'currency_usd' },

        'A7': { value: isId ? 'Sertifikat Keamanan SSL & RBAC' : 'SSL & RBAC Security Certificate', align: 'left' },
        'B7': { value: 1, align: 'center' },
        'C7': { value: isId ? 1800000 : 180, align: 'right', format: isId ? 'currency' : 'currency_usd' },
        'D7': { formula: '=B7*C7', bold: true, align: 'right', format: isId ? 'currency' : 'currency_usd' },

        'A9': { value: isId ? 'GRAND TOTAL ANGGARAN' : 'GRAND TOTAL BUDGET', bold: true, color: '#107c41' },
        'D9': { formula: '=SUM(D4:D7)', bold: true, bg: '#dcfce7', color: '#166534', align: 'right', format: isId ? 'currency' : 'currency_usd' }
      },
      [sheet2Name]: {
        'A1': { value: isId ? 'DAFTAR TIM & TUGAS' : 'TEAM & TASK LIST', bold: true, color: '#107c41', fontSize: 13 },
        'A3': { value: isId ? 'Nama Anggota' : 'Member Name', bold: true, bg: '#f1f5f9', color: '#334155' },
        'B3': { value: isId ? 'Peran / Role' : 'Role / Privilege', bold: true, bg: '#f1f5f9', color: '#334155' },
        'C3': { value: isId ? 'Status Task' : 'Task Status', bold: true, bg: '#f1f5f9', color: '#334155' },
        'A4': { value: 'dionipe' },
        'B4': { value: 'SuperAdmin' },
        'C4': { value: isId ? 'Selesai 100%' : 'Completed 100%' },
        'A5': { value: 'dionipeok' },
        'B5': { value: 'Admin' },
        'C5': { value: isId ? 'Dalam Proses' : 'In Progress' }
      }
    };
  },

  /* ------------------------------------------------------------------------
     Main Render Layout
     ------------------------------------------------------------------------ */
  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    this.container.innerHTML = `
      <div class="griidcraft-layout" style="display:flex; flex-direction:column; height:100%; user-select:none; background:var(--card-bg); font-family:-apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;">
        <!-- 1. Office-grade Ribbon Header & Toolbars -->
        <div class="wordcraft-ribbon-bar" style="background:var(--window-header-bg); border-bottom:1px solid var(--window-border); flex-shrink:0;">
          <!-- Ribbon Tab Navigation -->
          <div class="wordcraft-tabs-header" style="display:flex; align-items:center; height:34px; padding:0 8px; border-bottom:1px solid var(--window-border); gap:2px;">
            <button class="wordcraft-tab-btn tab-file" onclick="DokuGriidCraft.toggleFileMenu(event)" style="background:#107c41 !important;">
              ${isId ? 'Berkas' : 'File'} ▾
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'home' ? 'active' : ''}" onclick="DokuGriidCraft.switchTab('home')" style="${this.activeTab === 'home' ? 'color:#107c41; border-bottom-color:#107c41;' : ''}">
              ${isId ? 'Beranda' : 'Home'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'insert' ? 'active' : ''}" onclick="DokuGriidCraft.switchTab('insert')" style="${this.activeTab === 'insert' ? 'color:#107c41; border-bottom-color:#107c41;' : ''}">
              ${isId ? 'Sisipkan' : 'Insert'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'formulas' ? 'active' : ''}" onclick="DokuGriidCraft.switchTab('formulas')" style="${this.activeTab === 'formulas' ? 'color:#107c41; border-bottom-color:#107c41;' : ''}">
              ${isId ? 'Rumus' : 'Formulas'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'data' ? 'active' : ''}" onclick="DokuGriidCraft.switchTab('data')" style="${this.activeTab === 'data' ? 'color:#107c41; border-bottom-color:#107c41;' : ''}">
              ${isId ? 'Data' : 'Data'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'view' ? 'active' : ''}" onclick="DokuGriidCraft.switchTab('view')" style="${this.activeTab === 'view' ? 'color:#107c41; border-bottom-color:#107c41;' : ''}">
              ${isId ? 'Tampilan' : 'View'}
            </button>

            <div style="flex:1;"></div>

            <!-- Quick Action Bar -->
            <div style="display:flex; align-items:center; gap:6px; margin-right:8px;">
              <span id="griidcraft-status" style="font-size:11px; color:#107c41; font-weight:600; margin-right:4px;">● ${this.isDirty ? (isId ? 'Belum Disimpan' : 'Unsaved') : (isId ? 'Tersimpan' : 'Saved')}</span>
              <button class="mac-btn" onclick="DokuGriidCraft.openFromFileManagerModal()" style="height:24px; padding:0 8px; font-size:11px;" title="${isId ? 'Buka Lembar Kerja dari Finder' : 'Open from Finder'}">
                📂 ${isId ? 'Buka...' : 'Open...'}
              </button>
              <button class="mac-btn primary" onclick="DokuGriidCraft.saveSpreadsheet()" style="height:24px; padding:0 10px; font-size:11px;" title="${isId ? 'Simpan Lembar Kerja (Ctrl+S)' : 'Save (Ctrl+S)'}">
                💾 ${isId ? 'Simpan' : 'Save'}
              </button>
              <button class="mac-btn" onclick="DokuGriidCraft.openSaveAsModal()" style="height:24px; padding:0 8px; font-size:11px;" title="${isId ? 'Simpan Sebagai (Pilih Folder)' : 'Save As (Select Folder)'}">
                📑 ${isId ? 'Simpan Sebagai...' : 'Save As...'}
              </button>
            </div>
          </div>

          <!-- Dynamic Ribbon Panel based on active tab -->
          <div class="wordcraft-ribbon-panel" id="griidcraft-ribbon-content" style="padding:6px 12px 4px; min-height:84px; display:flex; align-items:stretch; gap:8px; overflow-x:auto;">
            ${this.renderActiveRibbonTab()}
          </div>
        </div>

        <!-- 2. Formula Bar -->
        ${this.showFormulaBar ? `
          <div class="griid-formula-bar" style="height:32px; background:var(--sidebar-bg); border-bottom:1px solid var(--window-border); display:flex; align-items:center; padding:0 8px; gap:4px; flex-shrink:0;">
            <!-- Name Box -->
            <div class="cell-coord-box" id="active-cell-display" style="width:60px; height:24px; display:flex; align-items:center; justify-content:center; font-weight:700; font-size:11.5px; background:var(--card-bg); border-radius:4px; border:1px solid var(--window-border); color:var(--text-main);">
              ${this.activeCell}
            </div>
            <div style="width:1px; height:18px; background:var(--window-border); margin:0 4px;"></div>
            <!-- Action buttons -->
            <button class="ribbon-btn" onclick="DokuGriidCraft.cancelFormulaEdit()" title="${isId ? 'Batal (Esc)' : 'Cancel (Esc)'}" style="height:24px; width:24px; color:#ef4444; font-size:11px;">✕</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.commitFormulaEdit()" title="${isId ? 'Terapkan (Enter)' : 'Apply (Enter)'}" style="height:24px; width:24px; color:#107c41; font-size:12px; font-weight:700;">✓</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.openFunctionWizard()" title="${isId ? 'Sisipkan Fungsi (fx)' : 'Insert Function (fx)'}" style="height:24px; width:28px; font-style:italic; font-weight:700; color:var(--text-muted); font-size:12px;">fx</button>
            <div style="width:1px; height:18px; background:var(--window-border); margin:0 4px;"></div>
            <!-- Formula Input -->
            <input type="text" class="formula-input" id="formula-bar-input" 
                   placeholder="${isId ? 'Ketik nilai atau rumus (contoh: =SUM(D4:D7), =B4*C4)' : 'Enter value or formula (e.g. =SUM(D4:D7), =B4*C4)'}"
                   style="flex:1; height:24px; border:1px solid var(--window-border); border-radius:4px; background:var(--card-bg); font-family:monospace; font-size:12px; color:var(--text-main); outline:none; padding:0 8px; box-shadow:inset 0 1px 2px rgba(0,0,0,0.04);"
                   oninput="DokuGriidCraft.onFormulaBarChange(this.value)"
                   onkeydown="if(event.key==='Enter') DokuGriidCraft.commitFormulaEdit()">
          </div>
        ` : ''}

        <!-- 3. Grid Container -->
        <div class="griid-sheet-scroll" id="griid-sheet-scroll" style="flex:1; overflow:auto; background:var(--card-bg); transform-origin:top left; transform:scale(${this.zoomLevel / 100});">
          ${this.renderGridTable()}
        </div>

        <!-- 4. Multi-Sheet Tabs Bar -->
        <div class="griidcraft-sheets-bar" style="height:32px; background:var(--sidebar-bg); border-top:1px solid var(--window-border); display:flex; align-items:center; padding:0 8px; gap:4px; flex-shrink:0;">
          <button class="ribbon-btn" style="height:22px; width:22px; padding:0; font-size:10px;" title="${isId ? 'Sheet Sebelumnya' : 'Previous Sheet'}">◀</button>
          <button class="ribbon-btn" style="height:22px; width:22px; padding:0; font-size:10px;" title="${isId ? 'Sheet Selanjutnya' : 'Next Sheet'}">▶</button>
          <div style="width:1px; height:16px; background:var(--window-border); margin:0 4px;"></div>
          <div style="display:flex; align-items:center; gap:2px; overflow-x:auto;">
            ${this.sheets.map(s => `
              <div class="griidcraft-sheet-tab ${s === this.activeSheet ? 'active' : ''}" 
                   onclick="DokuGriidCraft.switchSheet('${s}')" 
                   ondblclick="DokuGriidCraft.renameSheetPrompt('${s}')"
                   style="display:inline-flex; align-items:center; gap:6px; padding:4px 14px; border-radius:4px 4px 0 0; font-size:11.5px; font-weight:600; cursor:pointer; background:${s === this.activeSheet ? 'var(--card-bg)' : 'transparent'}; border:1px solid ${s === this.activeSheet ? 'var(--window-border)' : 'transparent'}; border-bottom:${s === this.activeSheet ? '2px solid #107c41' : 'none'}; color:${s === this.activeSheet ? '#107c41' : 'var(--text-muted)'};">
                <span>${s}</span>
                ${this.sheets.length > 1 ? `<span onclick="event.stopPropagation(); DokuGriidCraft.deleteSheetPrompt('${s}')" style="opacity:0.4; font-size:10px; margin-left:2px; cursor:pointer;" title="${isId ? 'Hapus sheet' : 'Delete sheet'}">✕</span>` : ''}
              </div>
            `).join('')}
          </div>
          <button class="ribbon-btn" onclick="DokuGriidCraft.addSheetPrompt()" style="height:22px; padding:0 8px; font-size:11px; margin-left:4px;" title="${isId ? 'Tambah Lembar Baru' : 'Add Sheet'}">
            + Sheet
          </button>
        </div>

        <!-- 5. Excel Status Bar -->
        <div class="griidcraft-statusbar" style="height:26px; background:var(--window-header-bg); border-top:1px solid var(--window-border); display:flex; align-items:center; justify-content:space-between; padding:0 12px; font-size:11px; color:var(--text-muted); flex-shrink:0;">
          <div style="display:flex; align-items:center; gap:16px;">
            <span style="font-weight:700; color:#107c41; letter-spacing:0.3px;">${isId ? 'SIAP' : 'READY'}</span>
            <span id="grid-stat-count">${isId ? 'Jumlah Sel' : 'Cell Count'}: 1</span>
            <span id="grid-stat-sum">Total: --</span>
            <span id="grid-stat-avg">${isId ? 'Rata-rata' : 'Average'}: --</span>
          </div>
          <div style="display:flex; align-items:center; gap:8px;">
            <button class="ribbon-btn" style="height:18px; width:18px; padding:0; font-size:10px;" onclick="DokuGriidCraft.adjustZoom(-10)">-</button>
            <input type="range" min="50" max="200" step="10" value="${this.zoomLevel}" oninput="DokuGriidCraft.setZoom(this.value)" style="width:70px; height:4px; cursor:pointer;">
            <button class="ribbon-btn" style="height:18px; width:18px; padding:0; font-size:10px;" onclick="DokuGriidCraft.adjustZoom(10)">+</button>
            <span id="griid-zoom-label" style="font-weight:600; min-width:32px; text-align:right;">${this.zoomLevel}%</span>
          </div>
        </div>
      </div>
    `;

    this.selectCell(this.activeCell);
    this.updateStatusBarMetrics();
  },

  /* ------------------------------------------------------------------------
     Ribbon Tabs Content
     ------------------------------------------------------------------------ */
  switchTab(tab) {
    this.activeTab = tab;
    const content = document.getElementById('griidcraft-ribbon-content');
    if (content) {
      content.innerHTML = this.renderActiveRibbonTab();
    }
    const tabsHeader = document.querySelector('.griidcraft-layout .wordcraft-tabs-header');
    if (tabsHeader) {
      tabsHeader.querySelectorAll('.wordcraft-tab-btn').forEach(btn => {
        if (!btn.classList.contains('tab-file')) {
          btn.classList.remove('active');
          btn.style.color = '';
          btn.style.borderBottomColor = '';
        }
      });
      const target = tabsHeader.querySelector(`.wordcraft-tab-btn[onclick*="${tab}"]`);
      if (target) {
        target.classList.add('active');
        target.style.color = '#107c41';
        target.style.borderBottomColor = '#107c41';
      }
    }
  },

  renderActiveRibbonTab() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const sheetData = this.data[this.activeSheet] || {};
    const curCell = sheetData[this.activeCell] || {};

    if (this.activeTab === 'home') {
      return `
        <!-- Clipboard Group -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.pasteCell()" title="${isId ? 'Tempel (Ctrl+V)' : 'Paste (Ctrl+V)'}" style="padding:0 8px; font-size:11px; font-weight:600; color:#107c41;">📋 ${isId ? 'Tempel' : 'Paste'}</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.cutCell()" title="${isId ? 'Potong (Ctrl+X)' : 'Cut (Ctrl+X)'}">✂️</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.copyCell()" title="${isId ? 'Salin (Ctrl+C)' : 'Copy (Ctrl+C)'}">📄</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.clearSelectedCell()" title="${isId ? 'Hapus Isi Sel' : 'Clear Cell Contents'}">🧹</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Papan Klip' : 'Clipboard'}</div>
        </div>

        <!-- Font Group -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <select class="ribbon-select" style="width:115px;" onchange="DokuGriidCraft.setCellFontFamily(this.value)">
              <option value="-apple-system, sans-serif" selected>SF Pro / System</option>
              <option value="Calibri, sans-serif">Calibri</option>
              <option value="Arial, sans-serif">Arial</option>
              <option value="Times New Roman, serif">Times New Roman</option>
              <option value="Courier New, monospace">Courier New</option>
            </select>
            <select class="ribbon-select" style="width:52px;" onchange="DokuGriidCraft.setCellFontSize(this.value)">
              <option value="11" selected>11 pt</option>
              <option value="9">9 pt</option>
              <option value="10">10 pt</option>
              <option value="12">12 pt</option>
              <option value="14">14 pt</option>
              <option value="16">16 pt</option>
              <option value="18">18 pt</option>
            </select>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn ${curCell.bold ? 'active' : ''}" onclick="DokuGriidCraft.toggleFormat('bold')" title="${isId ? 'Tebal (Ctrl+B)' : 'Bold (Ctrl+B)'}"><strong>B</strong></button>
            <button class="ribbon-btn ${curCell.italic ? 'active' : ''}" onclick="DokuGriidCraft.toggleFormat('italic')" title="${isId ? 'Miring (Ctrl+I)' : 'Italic (Ctrl+I)'}"><em>I</em></button>
            <button class="ribbon-btn ${curCell.underline ? 'active' : ''}" onclick="DokuGriidCraft.toggleFormat('underline')" title="${isId ? 'Garis Bawah (Ctrl+U)' : 'Underline (Ctrl+U)'}"><u>U</u></button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.toggleFormat('border')" title="${isId ? 'Semua Garis Batas' : 'All Borders'}">田</button>
            <label class="ribbon-btn" style="cursor:pointer; position:relative; overflow:hidden;" title="${isId ? 'Warna Teks' : 'Font Color'}">
              <span style="font-weight:700; border-bottom:3px solid #ef4444;">A</span>
              <input type="color" onchange="DokuGriidCraft.setCellColor(this.value)" style="opacity:0; position:absolute; inset:0; cursor:pointer;">
            </label>
            <label class="ribbon-btn" style="cursor:pointer; position:relative; overflow:hidden;" title="${isId ? 'Warna Latar Sel' : 'Fill Color'}">
              <span>🪣</span>
              <input type="color" onchange="DokuGriidCraft.setCellBg(this.value)" style="opacity:0; position:absolute; inset:0; cursor:pointer;">
            </label>
          </div>
          <div class="ribbon-group-title">${isId ? 'Font' : 'Font'}</div>
        </div>

        <!-- Alignment Group -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.setCellVAlign('top')" title="${isId ? 'Rata Atas' : 'Top Align'}">⤒</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.setCellVAlign('middle')" title="${isId ? 'Rata Tengah Vertikal' : 'Middle Align'}">⤓</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.setCellVAlign('bottom')" title="${isId ? 'Rata Bawah' : 'Bottom Align'}">⤓</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.toggleFormat('wrap')" title="${isId ? 'Bungkus Teks' : 'Wrap Text'}" style="font-size:11px; padding:0 6px;">↩ ${isId ? 'Bungkus' : 'Wrap'}</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn ${curCell.align === 'left' ? 'active' : ''}" onclick="DokuGriidCraft.setCellAlign('left')" title="${isId ? 'Rata Kiri' : 'Align Left'}">⇤</button>
            <button class="ribbon-btn ${curCell.align === 'center' ? 'active' : ''}" onclick="DokuGriidCraft.setCellAlign('center')" title="${isId ? 'Rata Tengah' : 'Align Center'}">↔</button>
            <button class="ribbon-btn ${curCell.align === 'right' ? 'active' : ''}" onclick="DokuGriidCraft.setCellAlign('right')" title="${isId ? 'Rata Kanan' : 'Align Right'}">⇥</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.mergeCells()" title="${isId ? 'Gabung Sel' : 'Merge & Center'}" style="font-size:11px; padding:0 6px;">⊞ ${isId ? 'Gabung' : 'Merge'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Perataan' : 'Alignment'}</div>
        </div>

        <!-- Number Formatting Group -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <select class="ribbon-select" style="width:140px;" onchange="DokuGriidCraft.setCellFormat(this.value)">
              <option value="general" ${!curCell.format ? 'selected' : ''}>${isId ? 'Umum (General)' : 'General'}</option>
              <option value="number" ${curCell.format === 'number' ? 'selected' : ''}>${isId ? 'Angka (1,234.56)' : 'Number (1,234.56)'}</option>
              <option value="currency" ${curCell.format === 'currency' ? 'selected' : ''}>${isId ? 'Mata Uang (Rp IDR)' : 'Currency (IDR Rp)'}</option>
              <option value="currency_usd" ${curCell.format === 'currency_usd' ? 'selected' : ''}>${isId ? 'Mata Uang ($ USD)' : 'Currency (USD $)'}</option>
              <option value="percent" ${curCell.format === 'percent' ? 'selected' : ''}>${isId ? 'Persentase (%)' : 'Percentage (%)'}</option>
              <option value="date" ${curCell.format === 'date' ? 'selected' : ''}>${isId ? 'Tanggal (DD/MM/YYYY)' : 'Date (DD/MM/YYYY)'}</option>
            </select>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn ${curCell.format === 'currency' ? 'active' : ''}" onclick="DokuGriidCraft.setCellFormat('currency')" title="${isId ? 'Format Rupiah' : 'Format Rupiah'}" style="font-weight:700; font-size:11px;">Rp</button>
            <button class="ribbon-btn ${curCell.format === 'currency_usd' ? 'active' : ''}" onclick="DokuGriidCraft.setCellFormat('currency_usd')" title="${isId ? 'Format Dolar' : 'Format Dollar'}" style="font-weight:700;">$</button>
            <button class="ribbon-btn ${curCell.format === 'percent' ? 'active' : ''}" onclick="DokuGriidCraft.setCellFormat('percent')" title="${isId ? 'Format Persentase' : 'Format Percentage'}" style="font-weight:700;">%</button>
            <button class="ribbon-btn ${curCell.format === 'number' ? 'active' : ''}" onclick="DokuGriidCraft.setCellFormat('number')" title="${isId ? 'Format Angka Desimal' : 'Format Decimal Number'}">,00</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.adjustDecimals(1)" title="${isId ? 'Tambah Desimal (.00)' : 'Increase Decimal (.00)'}">.00→</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.adjustDecimals(-1)" title="${isId ? 'Kurangi Desimal (.0)' : 'Decrease Decimal (.0)'}">←.0</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Angka' : 'Number'}</div>
        </div>

        <!-- Cells & Editing Group -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertAutoSum()" title="${isId ? 'Jumlah Otomatis (Σ)' : 'AutoSum (Σ)'}" style="font-weight:700; color:#107c41; padding:0 8px; font-size:11px;">Σ AutoSum</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.sortColumn('asc')" title="${isId ? 'Urutkan A-Z' : 'Sort A to Z'}">⬆️ A-Z</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.sortColumn('desc')" title="${isId ? 'Urutkan Z-A' : 'Sort Z to A'}">⬇️ Z-A</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertRowBelow()" title="${isId ? 'Sisipkan Baris di Bawah' : 'Insert Row Below'}">${isId ? '+ Baris' : '+ Row'}</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertColRight()" title="${isId ? 'Sisipkan Kolom di Kanan' : 'Insert Column Right'}">${isId ? '+ Kolom' : '+ Col'}</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.deleteCurrentRow()" title="${isId ? 'Hapus Baris Terpilih' : 'Delete Selected Row'}" style="color:#ef4444;">${isId ? '- Baris' : '- Row'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Pengeditan' : 'Editing'}</div>
        </div>
      `;
    }

    if (this.activeTab === 'insert') {
      return `
        <!-- Visual Charts Group -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.generateChartModal('bar')" style="padding:0 8px; font-size:11px; font-weight:600;">📊 ${isId ? 'Grafik Batang (Bar)' : 'Bar Chart'}</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.generateChartModal('line')" style="padding:0 8px; font-size:11px; font-weight:600;">📈 ${isId ? 'Grafik Garis (Line)' : 'Line Chart'}</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.generateChartModal('pie')" style="padding:0 8px; font-size:11px; font-weight:600;">🥧 ${isId ? 'Grafik Lingkaran (Pie)' : 'Pie Chart'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Grafik & Visual' : 'Charts'}</div>
        </div>

        <!-- Table & Functions Group -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertSampleTable()" style="padding:0 8px; font-size:11px;">▦ ${isId ? 'Sisipkan Tabel Standar' : 'Insert Sample Table'}</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.openFunctionWizard()" style="padding:0 8px; font-size:11px; font-style:italic;"><i>fx</i> ${isId ? 'Katalog Fungsi Matematika' : 'Function Library'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Tabel & Rumus' : 'Tables & Functions'}</div>
        </div>
      `;
    }

    if (this.activeTab === 'formulas') {
      return `
        <!-- Main Math Functions -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('SUM')" style="font-weight:700; color:#107c41;">Σ SUM</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('AVERAGE')">AVERAGE</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('COUNT')">COUNT</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('MAX')">MAX</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('MIN')">MIN</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('PRODUCT')">PRODUCT</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Fungsi Utama' : 'Math & Trig'}</div>
        </div>

        <!-- Logical & Text Functions -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('IF')">${isId ? '🔀 IF Logika' : '🔀 IF Logic'}</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('CONCAT')">CONCAT</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('VLOOKUP')">VLOOKUP</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.insertFormulaTemplate('ROUND')">ROUND</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Logika & Teks' : 'Logical & Text'}</div>
        </div>

        <!-- Recalculate -->
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.recalculateAllSheets()" style="padding:0 10px; font-size:11px; font-weight:600;">🔄 ${isId ? 'Hitung Ulang' : 'Calculate Now'}</button>
          </div>
          <div class="ribbon-row">
            <span style="font-size:10px; color:#107c41; font-weight:600;">⚡ ${isId ? 'Hitung Otomatis: AKTIF' : 'Auto-Calc: ON'}</span>
          </div>
          <div class="ribbon-group-title">${isId ? 'Penghitungan' : 'Calculation'}</div>
        </div>
      `;
    }

    if (this.activeTab === 'data') {
      return `
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.sortColumn('asc')">⬆️ ${isId ? 'Urutkan A ➔ Z' : 'Sort A to Z'}</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.sortColumn('desc')">⬇️ ${isId ? 'Urutkan Z ➔ A' : 'Sort Z to A'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Urutkan & Filter' : 'Sort & Filter'}</div>
        </div>

        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.exportCsv()">📄 ${isId ? 'Ekspor CSV' : 'Export CSV'}</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.exportTsv()">📑 ${isId ? 'Ekspor TSV' : 'Export TSV'}</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Format Tabular' : 'Export Tabular'}</div>
        </div>
      `;
    }

    if (this.activeTab === 'view') {
      return `
        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px; gap:8px;">
            <label style="display:flex; align-items:center; gap:4px; font-size:11px; cursor:pointer;">
              <input type="checkbox" ${this.showGridlines ? 'checked' : ''} onchange="DokuGriidCraft.toggleGridlines(this.checked)">
              <span>${isId ? 'Garis Kisi' : 'Gridlines'}</span>
            </label>
            <label style="display:flex; align-items:center; gap:4px; font-size:11px; cursor:pointer;">
              <input type="checkbox" ${this.showFormulaBar ? 'checked' : ''} onchange="DokuGriidCraft.toggleFormulaBar(this.checked)">
              <span>${isId ? 'Bilah Rumus' : 'Formula Bar'}</span>
            </label>
          </div>
          <div class="ribbon-row" style="gap:8px;">
            <label style="display:flex; align-items:center; gap:4px; font-size:11px; cursor:pointer;">
              <input type="checkbox" ${this.showHeadings ? 'checked' : ''} onchange="DokuGriidCraft.toggleHeadings(this.checked)">
              <span>${isId ? 'Judul Kolom/Baris' : 'Headings'}</span>
            </label>
          </div>
          <div class="ribbon-group-title">${isId ? 'Tampilan Lembar' : 'Show / Hide'}</div>
        </div>

        <div class="ribbon-group">
          <div class="ribbon-row" style="margin-bottom:3px;">
            <button class="ribbon-btn" onclick="DokuGriidCraft.setZoom(75)">75%</button>
            <button class="ribbon-btn ${this.zoomLevel === 100 ? 'active' : ''}" onclick="DokuGriidCraft.setZoom(100)">100%</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.setZoom(125)">125%</button>
          </div>
          <div class="ribbon-row">
            <button class="ribbon-btn" onclick="DokuGriidCraft.setZoom(150)">150%</button>
            <button class="ribbon-btn" onclick="DokuGriidCraft.setZoom(200)">200%</button>
          </div>
          <div class="ribbon-group-title">${isId ? 'Perbesaran' : 'Zoom'}</div>
        </div>
      `;
    }

    return '';
  },

  /* ------------------------------------------------------------------------
     File Dropdown Menu
     ------------------------------------------------------------------------ */
  toggleFileMenu(e) {
    e.stopPropagation();
    let menu = document.getElementById('griidcraft-file-menu');
    if (!menu) {
      menu = document.createElement('div');
      menu.id = 'griidcraft-file-menu';
      menu.style.position = 'fixed';
      menu.style.zIndex = '999999';
      menu.style.background = 'var(--window-bg)';
      menu.style.backdropFilter = 'blur(30px)';
      menu.style.border = '1px solid var(--window-border)';
      menu.style.borderRadius = '10px';
      menu.style.boxShadow = '0 16px 40px rgba(0,0,0,0.35)';
      menu.style.padding = '6px';
      menu.style.minWidth = '220px';
      document.body.appendChild(menu);

      window.addEventListener('click', () => {
        menu.style.display = 'none';
      });
    }

    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const rect = e.currentTarget.getBoundingClientRect();

    menu.innerHTML = `
      <div class="finder-menu-item" onclick="DokuGriidCraft.newBlankSpreadsheet()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>✨</span> <span>${isId ? 'Buku Kerja Baru' : 'New Blank Workbook'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuGriidCraft.openFromFileManagerModal()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📂</span> <span>${isId ? 'Buka dari Finder...' : 'Open from Finder...'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuGriidCraft.saveSpreadsheet()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>💾</span> <span>${isId ? 'Simpan ke BijiOS' : 'Save to BijiOS'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuGriidCraft.openSaveAsModal()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📑</span> <span>${isId ? 'Simpan Sebagai... (Pilih Folder)' : 'Save As... (Select Folder)'}</span>
      </div>
      <div style="height:1px; background:var(--window-border); margin:4px 0;"></div>
      <div style="font-size:10px; font-weight:700; color:var(--text-muted); padding:4px 12px; text-transform:uppercase;">${isId ? 'Ekspor Berkas' : 'Export File'}</div>
      <div class="finder-menu-item" onclick="DokuGriidCraft.exportXlsx()" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📊</span> <span>${isId ? 'Ekspor Excel (.xlsx)' : 'Export Excel (.xlsx)'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuGriidCraft.exportCsv()" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📄</span> <span>${isId ? 'Ekspor CSV (.csv)' : 'Export CSV (.csv)'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuGriidCraft.printOrExportPdf()" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📕</span> <span>${isId ? 'Cetak / Ekspor PDF' : 'Print / Export PDF'}</span>
      </div>
    `;

    menu.style.left = `${rect.left}px`;
    menu.style.top = `${rect.bottom + 4}px`;
    menu.style.display = 'block';
  },

  /* ------------------------------------------------------------------------
     Grid Rendering & Cell Evaluation
     ------------------------------------------------------------------------ */
  renderGridTable() {
    const sheetData = this.data[this.activeSheet] || {};
    const activeCol = this.activeCell ? this.activeCell[0] : '';
    const activeRow = this.activeCell ? parseInt(this.activeCell.substring(1)) : 0;

    return `
      <table class="excel-grid" id="excel-table" style="border-collapse:collapse; table-layout:fixed; font-size:12px; width:max-content;">
        ${this.showHeadings ? `
          <thead>
            <tr>
              <th style="width:46px; height:24px; background:var(--sidebar-bg); border:${this.showGridlines ? '1px solid var(--window-border)' : 'none'}; text-align:center; font-weight:700; color:var(--text-muted);"></th>
              ${this.cols.map(c => {
      const isColActive = (c === activeCol);
      return `
                  <th style="width:${this.colWidths[c] || 110}px; height:24px; background:${isColActive ? 'rgba(16,124,65,0.14)' : 'var(--sidebar-bg)'}; border:${this.showGridlines ? '1px solid var(--window-border)' : 'none'}; text-align:center; font-weight:700; color:${isColActive ? '#107c41' : 'var(--text-muted)'}; font-size:11.5px;">
                    ${c}
                  </th>
                `;
    }).join('')}
            </tr>
          </thead>
        ` : ''}
        <tbody>
          ${Array.from({ length: this.maxRows }).map((_, rIdx) => {
      const rNum = rIdx + 1;
      const isRowActive = (rNum === activeRow);
      return `
              <tr>
                ${this.showHeadings ? `
                  <th style="width:46px; height:24px; background:${isRowActive ? 'rgba(16,124,65,0.14)' : 'var(--sidebar-bg)'}; border:${this.showGridlines ? '1px solid var(--window-border)' : 'none'}; text-align:center; font-weight:600; color:${isRowActive ? '#107c41' : 'var(--text-muted)'}; font-size:11px;">
                    ${rNum}
                  </th>
                ` : ''}
                ${this.cols.map(c => {
        const coord = `${c}${rNum}`;
        const cell = sheetData[coord] || {};
        const displayVal = this.evaluateCell(coord, sheetData);

        const styles = [
          cell.bold ? 'font-weight:bold;' : '',
          cell.italic ? 'font-style:italic;' : '',
          cell.underline ? 'text-decoration:underline;' : '',
          cell.fontSize ? `font-size:${cell.fontSize}px;` : '',
          cell.fontFamily ? `font-family:${cell.fontFamily};` : '',
          cell.valign ? `vertical-align:${cell.valign};` : 'vertical-align:middle;',
          cell.color ? `color:${cell.color};` : '',
          cell.bg ? `background:${cell.bg};` : '',
          cell.align ? `text-align:${cell.align};` : '',
          this.showGridlines ? 'border:1px solid var(--window-border);' : 'border:none;'
        ].join('');

        const isSel = (coord === this.activeCell);

        return `
                    <td id="cell-${coord}" 
                        data-coord="${coord}" 
                        style="${styles} height:24px; padding:2px 8px; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; position:relative;"
                        class="${isSel ? 'selected' : ''}" 
                        onclick="DokuGriidCraft.selectCell('${coord}')" 
                        ondblclick="DokuGriidCraft.editCellDirect('${coord}')">
                      ${displayVal}
                    </td>
                  `;
      }).join('')}
              </tr>
            `;
    }).join('')}
        </tbody>
      </table>
    `;
  },

  renderGrid() {
    const scrollEl = document.getElementById('griid-sheet-scroll');
    if (scrollEl) {
      scrollEl.innerHTML = this.renderGridTable();
    }
  },

  /* ------------------------------------------------------------------------
     Cell Selection & Formula Bar Sync
     ------------------------------------------------------------------------ */
  selectCell(coord) {
    this.activeCell = coord;
    const display = document.getElementById('active-cell-display');
    const formulaInput = document.getElementById('formula-bar-input');
    if (display) display.textContent = coord;

    document.querySelectorAll('.excel-grid td').forEach(td => td.classList.remove('selected'));
    const targetTd = document.getElementById(`cell-${coord}`);
    if (targetTd) {
      targetTd.classList.add('selected');
    }

    const sheetData = this.data[this.activeSheet] || {};
    const cell = sheetData[coord];
    if (formulaInput) {
      formulaInput.value = cell ? (cell.formula || (cell.value !== undefined ? cell.value : '')) : '';
    }

    this.updateStatusBarMetrics();
  },

  onFormulaBarChange(val) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};

    this.isDirty = true;
    this.updateWindowTitle();
    this.updateStatusBarStatus('unsaved');

    if (val.startsWith('=')) {
      cell.formula = val;
      delete cell.value;
    } else {
      delete cell.formula;
      const num = parseFloat(val);
      cell.value = (isNaN(num) || val.trim() === '') ? val : num;
    }

    this.updateCellDisplay(this.activeCell);
    this.updateDependentCells(this.activeCell);
    this.updateStatusBarMetrics();
  },

  commitFormulaEdit() {
    const input = document.getElementById('formula-bar-input');
    if (input) {
      this.onFormulaBarChange(input.value);
    }
    // Move to next row
    const col = this.activeCell[0];
    const row = parseInt(this.activeCell.substring(1));
    if (row < this.maxRows) {
      this.selectCell(`${col}${row + 1}`);
    }
  },

  cancelFormulaEdit() {
    this.selectCell(this.activeCell);
  },

  editCellDirect(coord) {
    const td = document.getElementById(`cell-${coord}`);
    if (!td) return;
    const sheetData = this.data[this.activeSheet] || {};
    const cell = sheetData[coord] || {};
    const rawVal = cell.formula || (cell.value !== undefined ? cell.value : '');

    const input = document.createElement('input');
    input.type = 'text';
    input.value = rawVal;
    input.style.width = '100%';
    input.style.height = '100%';
    input.style.border = 'none';
    input.style.outline = 'none';
    input.style.font = 'inherit';
    input.style.background = 'transparent';
    input.style.color = 'inherit';

    td.innerHTML = '';
    td.appendChild(input);
    input.focus();

    const finish = () => {
      this.onFormulaBarChange(input.value);
      this.updateCellDisplay(coord);
    };

    input.addEventListener('blur', finish);
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        input.blur();
        const c = coord[0];
        const r = parseInt(coord.substring(1));
        if (r < this.maxRows) this.selectCell(`${c}${r + 1}`);
      } else if (e.key === 'Escape') {
        this.updateCellDisplay(coord);
      }
    });
  },

  /* ------------------------------------------------------------------------
     Formula Calculation Engine (ArtCraft / GridCraft Sovereign Engine)
     ------------------------------------------------------------------------ */
  evaluateCell(coord, sheetData) {
    const cell = sheetData[coord];
    if (!cell) return '';

    let evaluated = cell.value;

    if (cell.formula) {
      evaluated = this.computeFormula(cell.formula, sheetData);
    }

    if (evaluated === undefined || evaluated === null) return '';

    // Apply number format
    return this.formatCellValue(evaluated, cell.format, cell.decimals);
  },

  formatCellValue(val, format, decimals = 2) {
    if (typeof val !== 'number') return val;

    if (format === 'currency') {
      return new Intl.NumberFormat('id-ID', {
        style: 'currency',
        currency: 'IDR',
        maximumFractionDigits: 0
      }).format(val);
    }

    if (format === 'currency_usd') {
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: 'USD',
        minimumFractionDigits: 2
      }).format(val);
    }

    if (format === 'percent') {
      return `${(val * (val < 1 && val > -1 ? 100 : 1)).toFixed(1)}%`;
    }

    if (format === 'number') {
      return new Intl.NumberFormat('id-ID', {
        minimumFractionDigits: decimals,
        maximumFractionDigits: decimals
      }).format(val);
    }

    if (format === 'date') {
      return new Date().toLocaleDateString('id-ID');
    }

    return val;
  },

  computeFormula(formula, sheetData) {
    if (!formula.startsWith('=')) return formula;
    const f = formula.substring(1).trim();
    const upper = f.toUpperCase();

    // 1. =SUM(A1:A10) or =SUM(A1, B2, 100)
    if (upper.startsWith('SUM(') && upper.endsWith(')')) {
      const args = upper.substring(4, upper.length - 1);
      const values = this.resolveFormulaArgs(args, sheetData);
      return values.reduce((a, b) => a + b, 0);
    }

    // 2. =AVERAGE(A1:A10) or =AVG(A1:A10)
    if ((upper.startsWith('AVERAGE(') || upper.startsWith('AVG(')) && upper.endsWith(')')) {
      const idx = upper.indexOf('(');
      const args = upper.substring(idx + 1, upper.length - 1);
      const values = this.resolveFormulaArgs(args, sheetData);
      return values.length ? values.reduce((a, b) => a + b, 0) / values.length : 0;
    }

    // 3. =COUNT(A1:A10)
    if (upper.startsWith('COUNT(') && upper.endsWith(')')) {
      const args = upper.substring(6, upper.length - 1);
      const values = this.resolveFormulaArgs(args, sheetData);
      return values.length;
    }

    // 4. =MAX(A1:A10) / =MIN(A1:A10)
    if (upper.startsWith('MAX(') && upper.endsWith(')')) {
      const args = upper.substring(4, upper.length - 1);
      const values = this.resolveFormulaArgs(args, sheetData);
      return values.length ? Math.max(...values) : 0;
    }
    if (upper.startsWith('MIN(') && upper.endsWith(')')) {
      const args = upper.substring(4, upper.length - 1);
      const values = this.resolveFormulaArgs(args, sheetData);
      return values.length ? Math.min(...values) : 0;
    }

    // 5. =PRODUCT(A1:A10)
    if (upper.startsWith('PRODUCT(') && upper.endsWith(')')) {
      const args = upper.substring(8, upper.length - 1);
      const values = this.resolveFormulaArgs(args, sheetData);
      return values.reduce((a, b) => a * b, 1);
    }

    // 6. =ROUND(A1, 2)
    if (upper.startsWith('ROUND(') && upper.endsWith(')')) {
      const args = upper.substring(6, upper.length - 1).split(',');
      const val = parseFloat(this.evaluateCell(args[0].trim(), sheetData)) || 0;
      const dec = parseInt(args[1] ? args[1].trim() : 0);
      return Number(Math.round(val + 'e' + dec) + 'e-' + dec);
    }

    // 7. =IF(condition, trueVal, falseVal)
    if (upper.startsWith('IF(') && upper.endsWith(')')) {
      const inner = f.substring(3, f.length - 1);
      const parts = inner.split(',').map(s => s.trim());
      if (parts.length >= 2) {
        const condResult = this.evalCondition(parts[0], sheetData);
        return condResult ? parts[1].replace(/["']/g, '') : (parts[2] ? parts[2].replace(/["']/g, '') : '');
      }
    }

    // 8. =CONCAT(A1, " ", B1)
    if ((upper.startsWith('CONCAT(') || upper.startsWith('CONCATENATE(')) && upper.endsWith(')')) {
      const idx = upper.indexOf('(');
      const args = f.substring(idx + 1, f.length - 1).split(',');
      return args.map(arg => {
        const t = arg.trim();
        if ((t.startsWith('"') && t.endsWith('"')) || (t.startsWith("'") && t.endsWith("'"))) {
          return t.substring(1, t.length - 1);
        }
        return this.evaluateCell(t.toUpperCase(), sheetData);
      }).join('');
    }

    // 9. Arithmetic evaluation with cell coordinates e.g. =B4*C4 or =(A1+A2)/2
    try {
      const replaced = upper.replace(/([A-P])([0-9]{1,2})/g, (match) => {
        const val = this.getCellValue(match, sheetData);
        return typeof val === 'number' ? val : 0;
      });
      // Pure JS math expression parser
      return Function(`"use strict"; return (${replaced})`)();
    } catch {
      return '#VALUE!';
    }
  },

  evalCondition(condStr, sheetData) {
    try {
      const replaced = condStr.replace(/([A-P])([0-9]{1,2})/g, (match) => {
        const val = this.getCellValue(match, sheetData);
        return typeof val === 'number' ? val : `"${val}"`;
      });
      return Boolean(Function(`"use strict"; return (${replaced})`)());
    } catch {
      return false;
    }
  },

  resolveFormulaArgs(argsStr, sheetData) {
    const tokens = argsStr.split(',').map(s => s.trim());
    const results = [];

    tokens.forEach(tok => {
      if (tok.includes(':')) {
        results.push(...this.getRangeValues(tok, sheetData));
      } else {
        const num = parseFloat(tok);
        if (!isNaN(num)) {
          results.push(num);
        } else {
          const val = this.getCellValue(tok, sheetData);
          if (typeof val === 'number') results.push(val);
        }
      }
    });

    return results;
  },

  getCellValue(coord, sheetData) {
    const cell = sheetData[coord];
    if (!cell) return 0;
    if (cell.formula) return this.computeFormula(cell.formula, sheetData);
    return typeof cell.value === 'number' ? cell.value : parseFloat(cell.value) || 0;
  },

  getRangeValues(rangeStr, sheetData) {
    const parts = rangeStr.split(':');
    if (parts.length !== 2) return [];

    const startCol = parts[0][0];
    const startRow = parseInt(parts[0].substring(1));
    const endCol = parts[1][0];
    const endRow = parseInt(parts[1].substring(1));

    const vals = [];
    const colStartIdx = this.cols.indexOf(startCol);
    const colEndIdx = this.cols.indexOf(endCol);

    const minC = Math.min(colStartIdx, colEndIdx);
    const maxC = Math.max(colStartIdx, colEndIdx);
    const minR = Math.min(startRow, endRow);
    const maxR = Math.max(startRow, endRow);

    for (let c = minC; c <= maxC; c++) {
      for (let r = minR; r <= maxR; r++) {
        const val = this.getCellValue(`${this.cols[c]}${r}`, sheetData);
        if (typeof val === 'number' && !isNaN(val)) vals.push(val);
      }
    }
    return vals;
  },

  updateCellDisplay(coord) {
    const td = document.getElementById(`cell-${coord}`);
    if (td) {
      const sheetData = this.data[this.activeSheet] || {};
      td.textContent = this.evaluateCell(coord, sheetData);
    }
  },

  updateDependentCells(changedCoord) {
    const sheetData = this.data[this.activeSheet] || {};
    Object.keys(sheetData).forEach(coord => {
      const cell = sheetData[coord];
      if (cell.formula && cell.formula.toUpperCase().includes(changedCoord)) {
        this.updateCellDisplay(coord);
      }
    });
  },

  recalculateAllSheets() {
    this.render();
    if (window.DokuIsland) {
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      DokuIsland.success(isId ? 'Semua rumus dan sel berhasil dihitung ulang' : 'All formulas and cells recalculated', 'GridCraft');
    }
  },

  /* ------------------------------------------------------------------------
     Formatting Actions
     ------------------------------------------------------------------------ */
  toggleFormat(fmt) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell[fmt] = !cell[fmt];
    this.isDirty = true;
    this.updateWindowTitle();
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  setCellColor(color) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell.color = color;
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  setCellBg(bg) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell.bg = bg;
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  setCellAlign(align) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell.align = align;
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  setCellFormat(fmt) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell.format = fmt;
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  setCellFontFamily(font) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell.fontFamily = font;
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  setCellFontSize(size) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell.fontSize = size;
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  setCellVAlign(valign) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell.valign = valign;
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  adjustDecimals(dir) {
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell] = sheetData[this.activeCell] || {};
    cell.decimals = Math.max(0, Math.min(6, (cell.decimals !== undefined ? cell.decimals : 2) + dir));
    if (!cell.format) cell.format = 'number';
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  deleteCurrentRow() {
    const row = parseInt(this.activeCell.substring(1));
    const sheetData = this.data[this.activeSheet] || {};
    for (let r = row; r < this.maxRows; r++) {
      this.cols.forEach(c => {
        sheetData[`${c}${r}`] = sheetData[`${c}${r + 1}`];
      });
    }
    this.cols.forEach(c => {
      delete sheetData[`${c}${this.maxRows}`];
    });
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  insertAutoSum() {
    const col = this.activeCell[0];
    const row = parseInt(this.activeCell.substring(1));
    const startRow = Math.max(1, row - 4);
    const endRow = Math.max(1, row - 1);
    const formula = `=SUM(${col}${startRow}:${col}${endRow})`;
    this.onFormulaBarChange(formula);
    const input = document.getElementById('formula-bar-input');
    if (input) input.value = formula;
  },

  insertFormulaTemplate(type) {
    const col = this.activeCell[0];
    const row = parseInt(this.activeCell.substring(1));
    const startRow = Math.max(1, row - 3);
    const endRow = Math.max(1, row - 1);
    const formula = `=${type}(${col}${startRow}:${col}${endRow})`;
    this.onFormulaBarChange(formula);
    const input = document.getElementById('formula-bar-input');
    if (input) input.value = formula;
  },

  clearSelectedCell() {
    const sheetData = this.data[this.activeSheet] || {};
    delete sheetData[this.activeCell];
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  copyCell() {
    const sheetData = this.data[this.activeSheet] || {};
    this.clipboardCell = Object.assign({}, sheetData[this.activeCell] || {});
    if (window.DokuIsland) {
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      DokuIsland.info(isId ? `Sel ${this.activeCell} disalin ke clipboard` : `Cell ${this.activeCell} copied to clipboard`, 'GridCraft');
    }
  },

  cutCell() {
    this.copyCell();
    this.clearSelectedCell();
  },

  pasteCell() {
    if (!this.clipboardCell) return;
    const sheetData = this.data[this.activeSheet] = this.data[this.activeSheet] || {};
    sheetData[this.activeCell] = Object.assign({}, this.clipboardCell);
    this.isDirty = true;
    this.renderGrid();
    this.selectCell(this.activeCell);
  },

  insertRowBelow() {
    this.maxRows += 5;
    this.renderGrid();
  },

  insertColRight() {
    const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
    if (this.cols.length < alphabet.length) {
      this.cols.push(alphabet[this.cols.length]);
      this.renderGrid();
    }
  },

  /* ------------------------------------------------------------------------
     Multi-Sheet Management
     ------------------------------------------------------------------------ */
  switchSheet(name) {
    this.activeSheet = name;
    this.render();
  },

  addSheetPrompt() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const name = prompt(isId ? "Nama Lembar Kerja Baru:" : "New Sheet Name:", `Sheet ${this.sheets.length + 1}`);
    if (!name) return;
    if (this.sheets.includes(name)) {
      alert(isId ? "Nama sheet sudah ada!" : "Sheet name already exists!");
      return;
    }
    this.sheets.push(name);
    this.activeSheet = name;
    this.data[name] = {};
    this.isDirty = true;
    this.render();
  },

  renameSheetPrompt(oldName) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const newName = prompt(isId ? "Ubah nama sheet:" : "Rename sheet:", oldName);
    if (!newName || newName === oldName) return;
    const idx = this.sheets.indexOf(oldName);
    if (idx !== -1) {
      this.sheets[idx] = newName;
      this.data[newName] = this.data[oldName];
      delete this.data[oldName];
      this.activeSheet = newName;
      this.isDirty = true;
      this.render();
    }
  },

  deleteSheetPrompt(name) {
    if (this.sheets.length <= 1) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (!confirm(isId ? `Hapus sheet "${name}"?` : `Delete sheet "${name}"?`)) return;
    this.sheets = this.sheets.filter(s => s !== name);
    delete this.data[name];
    this.activeSheet = this.sheets[0];
    this.isDirty = true;
    this.render();
  },

  /* ------------------------------------------------------------------------
     View Controls (Gridlines, Headings, Formula Bar, Zoom)
     ------------------------------------------------------------------------ */
  toggleGridlines(checked) {
    this.showGridlines = checked;
    this.renderGrid();
  },

  toggleHeadings(checked) {
    this.showHeadings = checked;
    this.renderGrid();
  },

  toggleFormulaBar(checked) {
    this.showFormulaBar = checked;
    this.render();
  },

  setZoom(val) {
    this.zoomLevel = val;
    const scrollEl = document.getElementById('griid-sheet-scroll');
    const label = document.getElementById('griid-zoom-label');
    if (scrollEl) scrollEl.style.transform = `scale(${val / 100})`;
    if (label) label.textContent = `${val}%`;
  },

  adjustZoom(step) {
    const next = Math.min(200, Math.max(50, this.zoomLevel + step));
    this.setZoom(next);
  },

  /* ------------------------------------------------------------------------
     Status Bar Metrics & Status Indicator
     ------------------------------------------------------------------------ */
  updateStatusBarMetrics() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const sheetData = this.data[this.activeSheet] || {};
    const cell = sheetData[this.activeCell];
    const countEl = document.getElementById('grid-stat-count');
    const sumEl = document.getElementById('grid-stat-sum');
    const avgEl = document.getElementById('grid-stat-avg');

    if (countEl) countEl.textContent = `${isId ? 'Jumlah Sel' : 'Cell Count'}: 1`;

    if (cell && typeof cell.value === 'number') {
      const loc = isId ? 'id-ID' : 'en-US';
      if (sumEl) sumEl.textContent = `Total: ${cell.value.toLocaleString(loc)}`;
      if (avgEl) avgEl.textContent = `${isId ? 'Rata-rata' : 'Average'}: ${cell.value.toLocaleString(loc)}`;
    } else {
      if (sumEl) sumEl.textContent = `Total: --`;
      if (avgEl) avgEl.textContent = `${isId ? 'Rata-rata' : 'Average'}: --`;
    }
  },

  updateStatusBarStatus(type) {
    const el = document.getElementById('griidcraft-status');
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (el) {
      if (type === 'saved') {
        el.textContent = `● ${isId ? 'Tersimpan' : 'Saved'}`;
        el.style.color = '#10b981';
      } else {
        el.textContent = `● ${isId ? 'Belum Disimpan' : 'Unsaved'}`;
        el.style.color = '#ef4444';
      }
    }
  },

  /* ------------------------------------------------------------------------
     File Picker: Open Spreadsheet from Finder
     ------------------------------------------------------------------------ */
  async openFromFileManagerModal() {
    let overlay = document.getElementById('griidcraft-open-modal');
    if (!overlay) {
      overlay = document.createElement('div');
      overlay.id = 'griidcraft-open-modal';
      overlay.style.position = 'fixed';
      overlay.style.inset = '0';
      overlay.style.background = 'rgba(0,0,0,0.55)';
      overlay.style.backdropFilter = 'blur(12px)';
      overlay.style.zIndex = '999999';
      overlay.style.display = 'none';
      overlay.style.alignItems = 'center';
      overlay.style.justifyContent = 'center';
      document.body.appendChild(overlay);
    }

    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    overlay.style.display = 'flex';
    overlay.innerHTML = `
      <div style="width:580px; max-width:92vw; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:18px; box-shadow:0 30px 80px rgba(0,0,0,0.45); padding:24px; color:var(--text-main);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
          <div>
            <h3 style="font-size:17px; font-weight:700; display:flex; align-items:center; gap:8px;">
              <span>📂</span> ${isId ? 'Buka Lembar Kerja dari Finder' : 'Open Spreadsheet from Finder'}
            </h3>
            <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
              ${isId ? 'Pilih berkas spreadsheet (.xlsx, .csv, .tsv) di penyimpanan BijiOS Anda.' : 'Select spreadsheet file from your BijiOS workspace storage.'}
            </p>
          </div>
          <button class="traffic-btn btn-close" onclick="document.getElementById('griidcraft-open-modal').style.display='none'"></button>
        </div>

        <div style="margin-bottom:14px;">
          <input type="text" id="griidcraft-search-picker" 
                 placeholder="${isId ? '🔍 Cari nama spreadsheet...' : '🔍 Search spreadsheets...'}"
                 style="width:100%; padding:9px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:13px; outline:none;"
                 oninput="DokuGriidCraft.filterPickerList(this.value)">
        </div>

        <div id="griidcraft-picker-list" style="max-height:280px; overflow-y:auto; border:1px solid var(--window-border); border-radius:10px; background:var(--card-bg); padding:6px; margin-bottom:16px;">
          <div style="padding:20px; text-align:center; color:var(--text-muted); font-size:12px;">${isId ? 'Memuat berkas spreadsheet...' : 'Loading spreadsheets...'}</div>
        </div>

        <div style="display:flex; justify-content:space-between; align-items:center;">
          <button class="mac-btn" onclick="DokuGriidCraft.newBlankSpreadsheet(); document.getElementById('griidcraft-open-modal').style.display='none';">
            ✨ ${isId ? 'Buat Lembar Kerja Baru' : 'New Blank Sheet'}
          </button>
          <button class="mac-btn" onclick="document.getElementById('griidcraft-open-modal').style.display='none'">
            ${isId ? 'Tutup' : 'Close'}
          </button>
        </div>
      </div>
    `;

    try {
      const headers = {};
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const curUser = (window.DokuHub && DokuHub.currentUser) || null;
      if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

      const res = await fetch('/api/files?parent_id=all&category=all', { headers });
      if (res.ok) {
        const files = await res.json();
        const validExts = ['xlsx', 'csv', 'tsv', 'json'];
        this._availableDocs = (Array.isArray(files) ? files : []).filter(f => {
          const ext = (f.extension || '').toLowerCase();
          return validExts.includes(ext) || f.name.endsWith('.xlsx') || f.name.endsWith('.csv') || f.name.endsWith('.tsv');
        });
        this.renderPickerList(this._availableDocs);
      }
    } catch (e) {
      const listEl = document.getElementById('griidcraft-picker-list');
      if (listEl) listEl.innerHTML = `<div style="padding:20px; text-align:center; color:#ef4444; font-size:12px;">${isId ? 'Gagal memuat daftar berkas: ' : 'Failed to load file list: '}${e}</div>`;
    }
  },

  renderPickerList(docs) {
    const listEl = document.getElementById('griidcraft-picker-list');
    if (!listEl) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    if (!docs || docs.length === 0) {
      listEl.innerHTML = `
        <div style="padding:28px 16px; text-align:center; color:var(--text-muted); font-size:12px;">
          <div style="font-size:24px; margin-bottom:6px;">📊</div>
          ${isId ? 'Tidak ada berkas spreadsheet (.xlsx, .csv) yang ditemukan di folder ini.' : 'No spreadsheet files (.xlsx, .csv) found in this workspace.'}
        </div>
      `;
      return;
    }

    listEl.innerHTML = `
      <div style="display:flex; flex-direction:column; gap:4px;">
        ${docs.map(doc => {
      const sizeKb = typeof doc.size_bytes === 'number' ? (doc.size_bytes / 1024).toFixed(1) : '15.4';
      const folderName = (doc.parent_id === 'fld_projects') ? 'Projects' : (doc.parent_id === 'fld_docs' ? 'Documents' : 'Root');
      return `
            <div class="picker-item" onclick="DokuGriidCraft.openSelectedFromPicker('${doc.id}')"
                 style="display:flex; align-items:center; justify-content:space-between; padding:8px 12px; border-radius:8px; cursor:pointer; background:rgba(125,125,125,0.06); transition:background 0.15s;"
                 onmouseover="this.style.background='rgba(0,122,255,0.1)'"
                 onmouseout="this.style.background='rgba(125,125,125,0.06)'">
              <div style="display:flex; align-items:center; gap:10px; min-width:0;">
                <span style="font-size:20px;">📊</span>
                <div style="min-width:0;">
                  <div style="font-weight:600; font-size:13px; text-overflow:ellipsis; overflow:hidden; white-space:nowrap;">${doc.name}</div>
                  <div style="font-size:11px; color:var(--text-muted); display:flex; gap:6px;">
                    <span>📁 ${folderName}</span>
                    <span>•</span>
                    <span>${sizeKb} KB</span>
                  </div>
                </div>
              </div>
              <button class="mac-btn primary" style="font-size:11px; padding:3px 10px;" onclick="event.stopPropagation(); DokuGriidCraft.openSelectedFromPicker('${doc.id}')">
                ${isId ? 'Buka' : 'Open'}
              </button>
            </div>
          `;
    }).join('')}
      </div>
    `;
  },

  filterPickerList(query) {
    if (!this._availableDocs) return;
    const q = (query || '').toLowerCase().trim();
    const filtered = this._availableDocs.filter(d => d.name.toLowerCase().includes(q));
    this.renderPickerList(filtered);
  },

  async openSelectedFromPicker(docId) {
    const doc = this._availableDocs ? this._availableDocs.find(d => d.id === docId) : null;
    const modal = document.getElementById('griidcraft-open-modal');
    if (modal) modal.style.display = 'none';

    if (doc) {
      await this.loadFile(doc);
    } else {
      await this.loadFile({ id: docId, name: 'Spreadsheet.xlsx' });
    }
  },

  /* ------------------------------------------------------------------------
     Save As Modal: Save Spreadsheet to Selected Folder
     ------------------------------------------------------------------------ */
  async openSaveAsModal() {
    let overlay = document.getElementById('griidcraft-saveas-modal');
    if (!overlay) {
      overlay = document.createElement('div');
      overlay.id = 'griidcraft-saveas-modal';
      overlay.style.position = 'fixed';
      overlay.style.inset = '0';
      overlay.style.background = 'rgba(0,0,0,0.55)';
      overlay.style.backdropFilter = 'blur(12px)';
      overlay.style.zIndex = '999999';
      overlay.style.display = 'none';
      overlay.style.alignItems = 'center';
      overlay.style.justifyContent = 'center';
      document.body.appendChild(overlay);
    }

    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curName = (this.currentFile?.name && this.currentFile.id !== 'new_sheet')
      ? this.currentFile.name
      : (isId ? 'Buku Kerja Baru.xlsx' : 'Untitled Spreadsheet.xlsx');

    overlay.style.display = 'flex';
    overlay.innerHTML = `
      <div style="width:480px; max-width:92vw; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:18px; box-shadow:0 30px 80px rgba(0,0,0,0.45); padding:24px; color:var(--text-main);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
          <div>
            <h3 style="font-size:17px; font-weight:700; display:flex; align-items:center; gap:8px;">
              <span>💾</span> ${isId ? 'Simpan Sebagai (Save As)' : 'Save Spreadsheet As'}
            </h3>
            <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
              ${isId ? 'Tentukan nama berkas dan pilih folder tujuan penyimpanan.' : 'Set file name and choose destination folder.'}
            </p>
          </div>
          <button class="traffic-btn btn-close" onclick="document.getElementById('griidcraft-saveas-modal').style.display='none'"></button>
        </div>

        <div style="display:flex; flex-direction:column; gap:14px; margin-bottom:20px;">
          <div>
            <label style="font-size:12px; font-weight:600; display:block; margin-bottom:6px;">${isId ? 'Nama Spreadsheet (.xlsx):' : 'Spreadsheet Name (.xlsx):'}</label>
            <input type="text" id="griidcraft-saveas-name" value="${curName}" 
                   style="width:100%; padding:10px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:13px; outline:none;"
                   onkeydown="if(event.key==='Enter') DokuGriidCraft.submitSaveAs()">
          </div>

          <div>
            <label style="font-size:12px; font-weight:600; display:block; margin-bottom:6px;">${isId ? 'Folder Tujuan:' : 'Destination Folder:'}</label>
            <select id="griidcraft-saveas-folder" style="width:100%; padding:10px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:13px; outline:none;">
              <option value="fld_docs" ${this.currentFile?.parent_id === 'fld_docs' ? 'selected' : ''}>📁 ${isId ? 'Dokumen (Documents)' : 'Documents'}</option>
              <option value="fld_projects" ${this.currentFile?.parent_id === 'fld_projects' ? 'selected' : ''}>📁 ${isId ? 'Proyek (Projects)' : 'Projects'}</option>
              <option value="root" ${this.currentFile?.parent_id === 'root' ? 'selected' : ''}>🏠 ${isId ? 'Ruang Kerja Utama (Root Workspace)' : 'Root Workspace'}</option>
            </select>
          </div>
        </div>

        <div style="display:flex; justify-content:flex-end; gap:8px;">
          <button class="mac-btn" onclick="document.getElementById('griidcraft-saveas-modal').style.display='none'">${isId ? 'Batal' : 'Cancel'}</button>
          <button class="mac-btn primary" onclick="DokuGriidCraft.submitSaveAs()">💾 ${isId ? 'Simpan ke Folder' : 'Save Spreadsheet'}</button>
        </div>
      </div>
    `;
  },

  async submitSaveAs() {
    const nameInput = document.getElementById('griidcraft-saveas-name');
    const folderSelect = document.getElementById('griidcraft-saveas-folder');
    if (!nameInput) return;

    let filename = nameInput.value.trim();
    if (!filename) filename = 'Spreadsheet.xlsx';
    if (!filename.endsWith('.xlsx') && !filename.endsWith('.csv')) filename += '.xlsx';

    const parentId = folderSelect ? folderSelect.value : 'fld_docs';
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    const payload = {
      title: filename,
      sheets: this.sheets.map(name => ({
        name,
        cells: this.data[name] || {}
      }))
    };
    const content = JSON.stringify(payload, null, 2);

    try {
      const headers = { 'Content-Type': 'application/json' };
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const curUser = (window.DokuHub && DokuHub.currentUser) || null;
      if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

      const res = await fetch('/api/files', {
        method: 'POST',
        headers,
        body: JSON.stringify({
          name: filename,
          parent_id: parentId,
          mime_type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
          content: content,
          size: content.length
        })
      });

      const data = await res.json();
      if (res.ok && data.id) {
        this.currentFile = {
          id: data.id,
          name: filename,
          parent_id: parentId
        };
        this.isDirty = false;
        this.updateWindowTitle();
        this.updateStatusBarStatus('saved');

        const modal = document.getElementById('griidcraft-saveas-modal');
        if (modal) modal.style.display = 'none';

        if (window.DokuIsland) {
          const isId = (window.DokuI18n && DokuI18n.lang === 'id');
          DokuIsland.success(isId ? `Spreadsheet "${filename}" berhasil disimpan!` : `Spreadsheet "${filename}" saved successfully!`, 'GridCraft');
        }
        if (window.DokuFinder) window.DokuFinder.loadAllFiles();
      } else {
        alert((isId ? "Gagal menyimpan: " : "Failed to save: ") + (data.error || 'Error'));
      }
    } catch (e) {
      alert((isId ? "Gagal menyimpan spreadsheet: " : "Failed to save spreadsheet: ") + e);
    }
  },

  async saveSpreadsheet() {
    if (!this.currentFile || this.currentFile.id === 'new_sheet') {
      this.openSaveAsModal();
      return;
    }

    const payload = {
      title: this.currentFile.name,
      sheets: this.sheets.map(name => ({
        name,
        cells: this.data[name] || {}
      }))
    };
    const content = JSON.stringify(payload, null, 2);

    try {
      const headers = { 'Content-Type': 'application/json' };
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const curUser = (window.DokuHub && DokuHub.currentUser) || null;
      if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

      const res = await fetch(`/api/files/${this.currentFile.id}/content`, {
        method: 'PUT',
        headers,
        body: content
      });

      if (res.ok) {
        this.isDirty = false;
        this.updateWindowTitle();
        this.updateStatusBarStatus('saved');
        if (window.DokuIsland) {
          const isId = (window.DokuI18n && DokuI18n.lang === 'id');
          DokuIsland.success(isId ? `Spreadsheet "${this.currentFile.name}" berhasil disimpan!` : `Spreadsheet "${this.currentFile.name}" saved successfully!`, 'GridCraft');
        }
      } else {
        const err = await res.json();
        const isId = (window.DokuI18n && DokuI18n.lang === 'id');
        alert((isId ? "Gagal menyimpan: " : "Failed to save: ") + (err.error || 'Server error'));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  insertSampleTable() {
    this.initDefaultData();
    this.render();
    if (window.DokuIsland) {
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      DokuIsland.info(isId ? 'Tabel anggaran contoh disisipkan' : 'Sample budget table inserted', 'GridCraft');
    }
  },

  newBlankSpreadsheet() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (this.isDirty && !confirm(isId ? "Perubahan belum disimpan. Buat buku kerja baru?" : "Changes not saved. Create new workbook?")) return;

    this.currentFile = {
      id: 'new_sheet',
      name: isId ? 'Buku Kerja Baru.xlsx' : 'Untitled Spreadsheet.xlsx'
    };
    this.isDirty = false;
    this.sheets = ['Sheet 1'];
    this.activeSheet = 'Sheet 1';
    this.data = { 'Sheet 1': {} };
    this.updateWindowTitle();
    this.render();
  },

  /* ------------------------------------------------------------------------
     SVG Chart Generator Modal (Bar, Line, Pie)
     ------------------------------------------------------------------------ */
  generateChartModal(chartType) {
    const sheetData = this.data[this.activeSheet] || {};
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    // Extract items from row 4 to 10
    const labels = [];
    const values = [];

    for (let r = 1; r <= 20; r++) {
      const labelVal = sheetData[`A${r}`]?.value;
      const numVal = this.getCellValue(`D${r}`, sheetData) || this.getCellValue(`B${r}`, sheetData);
      if (labelVal && typeof numVal === 'number' && numVal > 0 && !String(labelVal).toUpperCase().includes('TOTAL')) {
        labels.push(String(labelVal));
        values.push(numVal);
      }
    }

    if (labels.length === 0) {
      labels.push('Server', 'Lisensi', 'Storage', 'Security');
      values.push(25000000, 22500000, 7500000, 1800000);
    }

    let modal = document.getElementById('griidcraft-chart-modal');
    if (!modal) {
      modal = document.createElement('div');
      modal.id = 'griidcraft-chart-modal';
      modal.style.position = 'fixed';
      modal.style.inset = '0';
      modal.style.background = 'rgba(0,0,0,0.55)';
      modal.style.backdropFilter = 'blur(15px)';
      modal.style.zIndex = '999999';
      modal.style.display = 'none';
      modal.style.alignItems = 'center';
      modal.style.justifyContent = 'center';
      document.body.appendChild(modal);
    }

    const maxVal = Math.max(...values, 1);
    let chartSvg = '';

    if (chartType === 'bar') {
      const barWidth = Math.floor(400 / labels.length) - 16;
      chartSvg = `
        <svg width="480" height="260" viewBox="0 0 480 260" style="overflow:visible;">
          <line x1="40" y1="210" x2="460" y2="210" stroke="rgba(255,255,255,0.2)" stroke-width="2" />
          ${values.map((v, idx) => {
        const h = Math.round((v / maxVal) * 160);
        const x = 50 + idx * (barWidth + 16);
        const y = 210 - h;
        const colors = ['#007aff', '#34c759', '#af52de', '#ff9500', '#ff2d55', '#5856d6'];
        const color = colors[idx % colors.length];
        return `
              <rect x="${x}" y="${y}" width="${barWidth}" height="${h}" rx="6" fill="${color}" opacity="0.88">
                <title>${labels[idx]}: ${v.toLocaleString('id-ID')}</title>
              </rect>
              <text x="${x + barWidth / 2}" y="${y - 8}" fill="#fff" font-size="10" text-anchor="middle" font-weight="700">
                ${v > 1000000 ? (v / 1000000).toFixed(1) + 'M' : v}
              </text>
              <text x="${x + barWidth / 2}" y="228" fill="rgba(255,255,255,0.7)" font-size="10" text-anchor="middle">
                ${labels[idx].slice(0, 10)}
              </text>
            `;
      }).join('')}
        </svg>
      `;
    } else if (chartType === 'line') {
      const points = values.map((v, idx) => {
        const x = 60 + idx * Math.floor(380 / Math.max(1, values.length - 1));
        const y = 210 - Math.round((v / maxVal) * 160);
        return `${x},${y}`;
      }).join(' ');

      chartSvg = `
        <svg width="480" height="260" viewBox="0 0 480 260">
          <line x1="40" y1="210" x2="460" y2="210" stroke="rgba(255,255,255,0.2)" stroke-width="2" />
          <polyline fill="none" stroke="#007aff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" points="${points}" />
          ${values.map((v, idx) => {
        const x = 60 + idx * Math.floor(380 / Math.max(1, values.length - 1));
        const y = 210 - Math.round((v / maxVal) * 160);
        return `
              <circle cx="${x}" cy="${y}" r="5" fill="#34c759" stroke="#fff" stroke-width="2" />
              <text x="${x}" y="${y - 10}" fill="#fff" font-size="10" text-anchor="middle" font-weight="700">
                ${v > 1000000 ? (v / 1000000).toFixed(1) + 'M' : v}
              </text>
              <text x="${x}" y="228" fill="rgba(255,255,255,0.7)" font-size="10" text-anchor="middle">
                ${labels[idx].slice(0, 8)}
              </text>
            `;
      }).join('')}
        </svg>
      `;
    } else {
      // Pie Chart
      const total = values.reduce((a, b) => a + b, 0);
      let cumulativeAngle = 0;
      const colors = ['#007aff', '#34c759', '#af52de', '#ff9500', '#ff2d55', '#5856d6'];

      const slices = values.map((v, idx) => {
        const sliceAngle = (v / total) * 360;
        const start = cumulativeAngle;
        cumulativeAngle += sliceAngle;
        const end = cumulativeAngle;

        const x1 = 120 + 90 * Math.cos(Math.PI * start / 180);
        const y1 = 120 + 90 * Math.sin(Math.PI * start / 180);
        const x2 = 120 + 90 * Math.cos(Math.PI * end / 180);
        const y2 = 120 + 90 * Math.sin(Math.PI * end / 180);
        const largeArc = sliceAngle > 180 ? 1 : 0;

        return `
          <path d="M 120 120 L ${x1} ${y1} A 90 90 0 ${largeArc} 1 ${x2} ${y2} Z" fill="${colors[idx % colors.length]}" opacity="0.9">
            <title>${labels[idx]}: ${((v / total) * 100).toFixed(1)}%</title>
          </path>
        `;
      }).join('');

      chartSvg = `
        <div style="display:flex; align-items:center; justify-content:center; gap:24px;">
          <svg width="240" height="240" viewBox="0 0 240 240">
            ${slices}
          </svg>
          <div style="display:flex; flex-direction:column; gap:6px;">
            ${labels.map((l, idx) => `
              <div style="display:flex; align-items:center; gap:8px; font-size:12px;">
                <span style="width:12px; height:12px; border-radius:3px; background:${colors[idx % colors.length]}; display:inline-block;"></span>
                <span>${l}: <strong>${((values[idx] / total) * 100).toFixed(1)}%</strong></span>
              </div>
            `).join('')}
          </div>
        </div>
      `;
    }

    modal.style.display = 'flex';
    modal.innerHTML = `
      <div style="width:560px; max-width:92vw; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:20px; box-shadow:0 30px 80px rgba(0,0,0,0.5); padding:24px; color:#ffffff;">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
          <h3 style="font-size:16px; font-weight:700; margin:0; display:flex; align-items:center; gap:8px;">
            <span>📊</span> ${isId ? 'Visualisasi Grafik Lembar Kerja' : 'Spreadsheet Chart Visualization'}
          </h3>
          <button class="traffic-btn btn-close" onclick="document.getElementById('griidcraft-chart-modal').style.display='none'"></button>
        </div>
        <div style="background:rgba(0,0,0,0.3); border-radius:14px; padding:18px; display:flex; justify-content:center; align-items:center; margin-bottom:18px; border:1px solid rgba(255,255,255,0.1);">
          ${chartSvg}
        </div>
        <div style="display:flex; justify-content:flex-end;">
          <button class="mac-btn primary" onclick="document.getElementById('griidcraft-chart-modal').style.display='none'">
            ${isId ? 'Tutup Pratinjau' : 'Close Preview'}
          </button>
        </div>
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     Function Wizard Modal
     ------------------------------------------------------------------------ */
  openFunctionWizard() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const funcs = [
      { name: 'SUM(range)', desc: isId ? 'Menghitung jumlah total semua angka dalam rentang sel.' : 'Calculates the sum of all numbers in the specified cell range.', ex: '=SUM(D4:D7)' },
      { name: 'AVERAGE(range)', desc: isId ? 'Menghitung nilai rata-rata aritmatika dari rentang sel.' : 'Calculates the arithmetic mean of a range of cells.', ex: '=AVERAGE(C4:C7)' },
      { name: 'COUNT(range)', desc: isId ? 'Menghitung banyaknya sel yang berisi angka numerik.' : 'Counts the number of cells containing numerical values.', ex: '=COUNT(B4:B7)' },
      { name: 'MAX(range)', desc: isId ? 'Mencari nilai tertinggi (maksimum) dalam rentang.' : 'Finds the maximum value within the cell range.', ex: '=MAX(C4:C7)' },
      { name: 'MIN(range)', desc: isId ? 'Mencari nilai terendah (minimum) dalam rentang.' : 'Finds the minimum value within the cell range.', ex: '=MIN(C4:C7)' },
      { name: 'IF(cond, val_true, val_false)', desc: isId ? 'Evaluasi kondisi logika bernilai benar atau salah.' : 'Evaluates a logical condition and returns true/false result.', ex: '=IF(B4>1, "Banyak", "Sedikit")' },
      { name: 'CONCAT(text1, text2)', desc: isId ? 'Menggabungkan beberapa string teks menjadi satu teks utuh.' : 'Concatenates multiple text strings into a single string.', ex: '=CONCAT(A4, " - OK")' },
      { name: 'ROUND(number, decimals)', desc: isId ? 'Membulatkan angka ke jumlah desimal tertentu.' : 'Rounds a number to a specified number of decimal digits.', ex: '=ROUND(125.456, 2)' },
      { name: 'PRODUCT(range)', desc: isId ? 'Mengalikan seluruh angka dalam rentang sel.' : 'Multiplies all numbers given in the cell range.', ex: '=PRODUCT(B4, C4)' }
    ];

    let modal = document.getElementById('griidcraft-fx-modal');
    if (!modal) {
      modal = document.createElement('div');
      modal.id = 'griidcraft-fx-modal';
      modal.style.position = 'fixed';
      modal.style.inset = '0';
      modal.style.background = 'rgba(0,0,0,0.55)';
      modal.style.backdropFilter = 'blur(15px)';
      modal.style.zIndex = '999999';
      modal.style.display = 'none';
      modal.style.alignItems = 'center';
      modal.style.justifyContent = 'center';
      document.body.appendChild(modal);
    }

    modal.style.display = 'flex';
    modal.innerHTML = `
      <div style="width:540px; max-width:92vw; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:18px; box-shadow:0 30px 80px rgba(0,0,0,0.5); padding:24px; color:var(--text-main);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
          <h3 style="font-size:16px; font-weight:700; margin:0; display:flex; align-items:center; gap:8px;">
            <span><i>fx</i></span> ${isId ? 'Katalog Fungsi Matematika & Rumus' : 'Function Library'}
          </h3>
          <button class="traffic-btn btn-close" onclick="document.getElementById('griidcraft-fx-modal').style.display='none'"></button>
        </div>
        <div style="max-height:300px; overflow-y:auto; border:1px solid var(--window-border); border-radius:10px; background:var(--card-bg); padding:6px; margin-bottom:16px;">
          ${funcs.map(f => `
            <div onclick="DokuGriidCraft.insertFxFromWizard('${f.ex}')" 
                 style="padding:10px 12px; border-radius:8px; cursor:pointer; background:rgba(125,125,125,0.06); margin-bottom:4px; transition:background 0.15s;"
                 onmouseover="this.style.background='rgba(0,122,255,0.1)'"
                 onmouseout="this.style.background='rgba(125,125,125,0.06)'">
              <div style="display:flex; justify-content:space-between; align-items:center;">
                <span style="font-weight:700; font-family:monospace; color:var(--accent-color);">${f.name}</span>
                <span style="font-size:11px; background:rgba(0,122,255,0.15); padding:2px 8px; border-radius:6px; font-family:monospace;">${f.ex}</span>
              </div>
              <div style="font-size:11.5px; color:var(--text-muted); margin-top:4px;">${f.desc}</div>
            </div>
          `).join('')}
        </div>
        <div style="display:flex; justify-content:flex-end;">
          <button class="mac-btn" onclick="document.getElementById('griidcraft-fx-modal').style.display='none'">
            ${isId ? 'Tutup' : 'Close'}
          </button>
        </div>
      </div>
    `;
  },

  insertFxFromWizard(formula) {
    this.onFormulaBarChange(formula);
    const input = document.getElementById('formula-bar-input');
    if (input) input.value = formula;
    const modal = document.getElementById('griidcraft-fx-modal');
    if (modal) modal.style.display = 'none';
  },

  /* ------------------------------------------------------------------------
     Export Engine (.xlsx, .csv, .tsv, print)
     ------------------------------------------------------------------------ */
  async exportXlsx() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const filename = this.currentFile.name.endsWith('.xlsx') ? this.currentFile.name : `${this.currentFile.name}.xlsx`;
    const payload = {
      title: filename,
      sheets: this.sheets.map(name => ({
        name,
        cells: this.data[name] || {}
      }))
    };

    try {
      if (window.DokuIsland) {
        DokuIsland.info(isId ? `Menyiapkan berkas OpenXML "${filename}"...` : `Preparing OpenXML file "${filename}"...`, 'GridCraft');
      }
      const headers = { 'Content-Type': 'application/json' };
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const curUser = (window.DokuHub && DokuHub.currentUser) || null;
      if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

      const res = await fetch('/api/files/export_xlsx', {
        method: 'POST',
        headers,
        body: JSON.stringify(payload)
      });

      if (!res.ok) throw new Error(`HTTP error ${res.status}`);

      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      if (window.DokuIsland) {
        DokuIsland.success(isId ? `Spreadsheet "${filename}" berhasil diekspor!` : `Spreadsheet "${filename}" exported successfully!`, 'GridCraft');
      }
    } catch (e) {
      console.error('Failed to export OpenXML XLSX:', e);
      if (window.DokuIsland) {
        DokuIsland.error(isId ? `Gagal mengekspor berkas XLSX` : `Failed to export XLSX file`, 'GridCraft');
      }
    }
  },

  exportCsv() {
    const sheetData = this.data[this.activeSheet] || {};
    let csv = '';

    for (let r = 1; r <= this.maxRows; r++) {
      let hasData = false;
      const row = this.cols.map(c => {
        const val = this.evaluateCell(`${c}${r}`, sheetData);
        if (val !== '') hasData = true;
        return `"${String(val).replace(/"/g, '""')}"`;
      });
      if (hasData) {
        csv += row.join(',') + '\n';
      }
    }

    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${this.activeSheet}.csv`;
    a.click();
  },

  exportTsv() {
    const sheetData = this.data[this.activeSheet] || {};
    let tsv = '';

    for (let r = 1; r <= this.maxRows; r++) {
      let hasData = false;
      const row = this.cols.map(c => {
        const val = this.evaluateCell(`${c}${r}`, sheetData);
        if (val !== '') hasData = true;
        return String(val).replace(/\t/g, ' ');
      });
      if (hasData) {
        tsv += row.join('\t') + '\n';
      }
    }

    const blob = new Blob([tsv], { type: 'text/tab-separated-values;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${this.activeSheet}.tsv`;
    a.click();
  },

  printOrExportPdf() {
    window.print();
  }
};
