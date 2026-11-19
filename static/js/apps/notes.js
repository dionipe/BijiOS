/**
 * DokuNotes & Markdown Document Studio for DokuHub
 * Features:
 * - Full Markdown Document (.md) support: open/edit/save directly with Finder or manage quick Notes
 * - Live Auto Preview (Split-Screen Editor + Rendered Preview, Editor-Only, Preview-Only)
 * - Rich Formatting Icon Toolbar (H1-H3, Bold, Italic, Strike, Highlight, Lists, Checklist, Quote, Code, Table, Link, Image, Callout, HR)
 * - Synchronized scrolling & live word/char/reading-time status bar
 * - Export to .md file, Save to Finder (.md), and Print/PDF
 * - 100% Bilingual (ID & EN) & Adaptive Light/Dark Mode
 */

window.DokuNotes = {
  container: null,
  notes: [],
  activeNoteId: null,
  activeFile: null, // When editing a .md file from Finder
  fileContent: '',
  viewMode: 'split', // 'split' (Auto Preview) | 'edit' | 'preview'
  showSidebar: true,
  isDirty: false,
  _saveTimeout: null,

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
    await this.loadNotes();

    if (file && file.id) {
      await this.openFinderFile(file);
    } else {
      this.activeFile = null;
      this.render();
      this.updateWindowTitle();
    }
  },

  async openFinderFile(file) {
    this.activeFile = file;
    this.isDirty = false;
    this.fileContent = '';

    try {
      const res = await fetch(`/api/files/${file.id}/content`, {
        headers: this.getAuthHeaders()
      });
      if (res.ok) {
        const text = await res.text();
        // If stored as JSON wrapper, unwrap content; otherwise use raw markdown text
        if (text.trim().startsWith('{')) {
          try {
            const parsed = JSON.parse(text);
            this.fileContent = parsed.content !== undefined ? parsed.content : text;
          } catch {
            this.fileContent = text;
          }
        } else {
          this.fileContent = text;
        }
      }
    } catch (e) {
      console.warn('Could not load markdown file content:', e);
    }

    if (!this.fileContent && file.content) {
      this.fileContent = file.content;
    }

    this.render();
    this.updateWindowTitle();
  },

  updateWindowTitle() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const win = document.getElementById('window-notes');
    if (!win) return;
    const titleEl = win.querySelector('.window-title');
    if (titleEl) {
      if (this.activeFile) {
        titleEl.innerHTML = `Ⓜ️ Markdown — ${this.activeFile.name}${this.isDirty ? ' <span style="color:#ef4444; font-size:10px;">●</span>' : ''}`;
      } else {
        titleEl.innerHTML = `🗒️ ${isId ? 'Notes & Dokumen Markdown' : 'Notes & Markdown Studio'}`;
      }
    }
  },

  async loadNotes() {
    try {
      const res = await fetch('/api/notes', { headers: this.getAuthHeaders() });
      if (res.ok) {
        this.notes = await res.json();
        if (this.notes.length > 0 && !this.activeNoteId) {
          this.activeNoteId = this.notes[0].id;
        }
      }
    } catch {
      this.notes = [];
    }
  },

  getActiveContent() {
    if (this.activeFile) return this.fileContent || '';
    const activeNote = this.notes.find(n => n.id === this.activeNoteId) || this.notes[0];
    return activeNote ? (activeNote.content || '') : '';
  },

  getActiveTitle() {
    if (this.activeFile) return this.activeFile.name || 'Untitled.md';
    const activeNote = this.notes.find(n => n.id === this.activeNoteId) || this.notes[0];
    return activeNote ? (activeNote.title || '') : '';
  },

  renderSidebar() {
    this.render();
  },

  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const activeNote = this.notes.find(n => n.id === this.activeNoteId) || this.notes[0] || { title: '', content: '' };
    const currentTitle = this.getActiveTitle();
    const currentContent = this.getActiveContent();

    const showEditor = this.viewMode === 'split' || this.viewMode === 'edit';
    const showPreview = this.viewMode === 'split' || this.viewMode === 'preview';

    this.container.innerHTML = `
      <div class="notes-layout" style="display:flex; height:100%; background:var(--window-bg); color:var(--text-main); font-family:-apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif; overflow:hidden;">
        <!-- Left Sidebar: Notes List -->
        ${this.showSidebar ? `
          <div class="notes-list-col" style="width:230px; min-width:210px; background:var(--sidebar-bg); border-right:1px solid var(--window-border); padding:12px 10px; display:flex; flex-direction:column; overflow-y:auto; flex-shrink:0;">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:10px; padding:0 2px;">
              <strong style="font-size:13px; color:var(--text-main);">Ⓜ️ ${isId ? 'Dokumen' : 'Docs'} (${this.notes.length})</strong>
              <button class="mac-btn primary" style="padding:3px 9px; font-size:11px;" onclick="DokuNotes.createNote()" title="${isId ? 'Buat Dokumen Markdown Baru' : 'New Markdown Document'}">
                + ${isId ? 'Baru' : 'New'}
              </button>
            </div>

            ${this.activeFile ? `
              <div style="padding:8px 10px; border-radius:8px; background:rgba(0,122,255,0.14); border:1px solid var(--accent-color); margin-bottom:10px;">
                <div style="font-size:10px; font-weight:700; color:var(--accent-color); text-transform:uppercase; margin-bottom:2px;">📁 ${isId ? 'Berkas Finder Aktif' : 'Active Finder File'}</div>
                <div style="font-size:12.5px; font-weight:700; color:var(--text-main); white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">${this.activeFile.name}</div>
              </div>
            ` : ''}

            <div style="display:flex; flex-direction:column; gap:6px;">
              ${this.notes.map(n => `
                <div class="note-item-card ${(!this.activeFile && n.id === this.activeNoteId) ? 'active' : ''}" onclick="DokuNotes.selectNote('${n.id}')">
                  <div class="note-item-title">
                    ${n.pinned ? '📌 ' : ''}${this.escapeHtml(n.title || (isId ? 'Tanpa Judul' : 'Untitled'))}
                  </div>
                  <div class="note-item-preview">
                    ${this.escapeHtml(n.content ? n.content.replace(/[#*_`>\[\]]/g, '').substring(0, 65) : (isId ? 'Dokumen kosong...' : 'Empty document...'))}
                  </div>
                </div>
              `).join('')}
            </div>
          </div>
        ` : ''}

        <!-- Right Main Markdown Studio -->
        <div style="flex:1; display:flex; flex-direction:column; min-width:0; overflow:hidden;">
          <!-- Top Title & Action Header -->
          <div style="padding:8px 14px; background:var(--window-bg); border-bottom:1px solid var(--window-border); display:flex; align-items:center; justify-content:space-between; gap:10px; flex-shrink:0;">
            <div style="display:flex; align-items:center; gap:8px; flex:1; min-width:0;">
              <button class="md-tool-btn" onclick="DokuNotes.toggleSidebar()" title="${isId ? 'Tampilkan/Sembunyikan Sidebar' : 'Toggle Sidebar'}">◧</button>
              <input type="text" id="note-title-field"
                     value="${this.escapeAttr(currentTitle)}"
                     ${this.activeFile ? 'readonly' : ''}
                     placeholder="${isId ? 'Judul Dokumen Markdown...' : 'Markdown Document Title...'}"
                     oninput="DokuNotes.onNoteChange()"
                     style="flex:1; min-width:120px; border:none; outline:none; background:transparent; font-size:15px; font-weight:700; color:var(--text-main);">
            </div>

            <div style="display:flex; align-items:center; gap:6px; flex-shrink:0;">
              <span id="note-save-status" style="font-size:11px; color:var(--text-muted); margin-right:4px;">
                ● ${isId ? 'Tersimpan' : 'Saved'}
              </span>

              <!-- View Mode Switcher -->
              <div style="display:inline-flex; background:var(--sidebar-bg); border:1px solid var(--window-border); border-radius:7px; padding:2px; gap:2px;">
                <button class="md-tool-btn ${this.viewMode === 'edit' ? 'active' : ''}" onclick="DokuNotes.setViewMode('edit')" title="${isId ? 'Mode Editor Saja' : 'Editor Only'}" style="height:22px; font-size:11px;">
                  ✏️ ${isId ? 'Edit' : 'Edit'}
                </button>
                <button class="md-tool-btn ${this.viewMode === 'split' ? 'active' : ''}" onclick="DokuNotes.setViewMode('split')" title="${isId ? 'Split Auto Preview' : 'Split Auto Preview'}" style="height:22px; font-size:11px;">
                  ◫ ${isId ? 'Auto Preview' : 'Auto Preview'}
                </button>
                <button class="md-tool-btn ${this.viewMode === 'preview' ? 'active' : ''}" onclick="DokuNotes.setViewMode('preview')" title="${isId ? 'Mode Pratinjau Penuh' : 'Preview Only'}" style="height:22px; font-size:11px;">
                  👁️ ${isId ? 'Preview' : 'Preview'}
                </button>
              </div>

              ${this.activeFile ? `
                <button class="mac-btn primary" onclick="DokuNotes.saveFinderFile()" style="padding:4px 10px; font-size:11px;" title="${isId ? 'Simpan Berkas .md ke Finder' : 'Save .md File to Finder'}">
                  💾 ${isId ? 'Simpan' : 'Save'}
                </button>
              ` : `
                <button class="mac-btn" onclick="DokuNotes.saveAsFinderMd()" style="padding:4px 9px; font-size:11px;" title="${isId ? 'Simpan sebagai berkas .md di Finder' : 'Save as .md in Finder'}">
                  📁 ${isId ? 'Simpan .md' : 'Save .md'}
                </button>
                <button class="mac-btn" onclick="DokuNotes.togglePin()" style="padding:4px 8px; font-size:11px;" title="${isId ? 'Sematkan' : 'Pin'}">
                  ${activeNote.pinned ? '📌' : '📍'}
                </button>
                <button class="mac-btn" style="color:#ef4444; padding:4px 8px; font-size:11px;" onclick="DokuNotes.deleteActiveNote()" title="${isId ? 'Hapus' : 'Delete'}">
                  🗑️
                </button>
              `}
            </div>
          </div>

          <!-- Markdown Formatting Icon Toolbar -->
          <div class="md-toolbar">
            <button class="md-tool-btn" onclick="DokuNotes.insertHeading(1)" title="${isId ? 'Judul Besar (H1)' : 'Heading 1'}">H1</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertHeading(2)" title="${isId ? 'Subjudul (H2)' : 'Heading 2'}">H2</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertHeading(3)" title="${isId ? 'Sub-subjudul (H3)' : 'Heading 3'}">H3</button>
            <div class="md-tool-sep"></div>

            <button class="md-tool-btn" onclick="DokuNotes.wrapSelection('**', '**', '${isId ? 'teks tebal' : 'bold text'}')" title="${isId ? 'Tebal (Bold)' : 'Bold (Ctrl+B)'}"><strong>B</strong></button>
            <button class="md-tool-btn" onclick="DokuNotes.wrapSelection('*', '*', '${isId ? 'teks miring' : 'italic text'}')" title="${isId ? 'Miring (Italic)' : 'Italic (Ctrl+I)'}"><em>I</em></button>
            <button class="md-tool-btn" onclick="DokuNotes.wrapSelection('~~', '~~', '${isId ? 'coret' : 'strikethrough'}')" title="${isId ? 'Coret (Strikethrough)' : 'Strikethrough'}"><del>S</del></button>
            <button class="md-tool-btn" onclick="DokuNotes.wrapSelection('==', '==', '${isId ? 'sorotan' : 'highlight'}')" title="${isId ? 'Sorot Teks (Highlight)' : 'Highlight Text'}">🖍️</button>
            <div class="md-tool-sep"></div>

            <button class="md-tool-btn" onclick="DokuNotes.insertLinePrefix('- ')" title="${isId ? 'Daftar Poin (Bullet List)' : 'Bullet List'}">• ≡</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertLinePrefix('1. ')" title="${isId ? 'Daftar Bernomor (Numbered List)' : 'Numbered List'}">1. ≡</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertLinePrefix('- [ ] ')" title="${isId ? 'Daftar Tugas (Checklist)' : 'Task Checklist'}">☑</button>
            <div class="md-tool-sep"></div>

            <button class="md-tool-btn" onclick="DokuNotes.insertLinePrefix('> ')" title="${isId ? 'Kutipan (Blockquote)' : 'Blockquote'}">❝</button>
            <button class="md-tool-btn" onclick="DokuNotes.wrapSelection('\`', '\`', 'kode')" title="${isId ? 'Kode Sebaris (Inline Code)' : 'Inline Code'}">&lt;/&gt;</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertCodeBlock()" title="${isId ? 'Blok Kode (Code Block)' : 'Code Block'}">⌨️</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertTable()" title="${isId ? 'Sisipkan Tabel Markdown' : 'Insert Markdown Table'}">⊞</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertSnippet('\\n---\\n')" title="${isId ? 'Garis Pemisah (Horizontal Rule)' : 'Horizontal Rule'}">—</button>
            <div class="md-tool-sep"></div>

            <button class="md-tool-btn" onclick="DokuNotes.insertLink()" title="${isId ? 'Sisipkan Tautan (Link)' : 'Insert Link'}">🔗</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertImage()" title="${isId ? 'Sisipkan Gambar (Image)' : 'Insert Image'}">🖼️</button>
            <button class="md-tool-btn" onclick="DokuNotes.insertCallout()" title="${isId ? 'Kotak Catatan (Callout)' : 'Insert Callout Box'}">💡</button>

            <div style="flex:1;"></div>

            <button class="md-tool-btn" onclick="DokuNotes.downloadMdFile()" title="${isId ? 'Unduh Berkas .md' : 'Download .md File'}">⬇️</button>
            <button class="md-tool-btn" onclick="DokuNotes.printPreview()" title="${isId ? 'Cetak / Ekspor PDF' : 'Print / Export PDF'}">🖨️</button>
          </div>

          <!-- Split Workspace: Editor + Auto Preview -->
          <div class="md-workspace">
            ${showEditor ? `
              <div class="md-editor-pane">
                <textarea class="md-editor-textarea" id="note-content-field"
                          placeholder="${isId ? 'Mulai menulis dokumen Markdown Anda di sini...\n\nGunakan ikon di atas atau ketik # untuk Judul, **tebal**, *miring*, - [ ] checklist, dll.' : 'Start writing your Markdown document here...\n\nUse the toolbar icons above or type # Heading, **bold**, *italic*, - [ ] checklist, etc.'}"
                          oninput="DokuNotes.onNoteChange()"
                          onscroll="DokuNotes.syncScroll(this)"
                          onkeydown="DokuNotes.handleEditorKeydown(event)">${this.escapeHtml(currentContent)}</textarea>
              </div>
            ` : ''}

            ${showPreview ? `
              <div class="md-preview-pane" id="md-live-preview">
                ${this.parseMarkdown(currentContent)}
              </div>
            ` : ''}
          </div>

          <!-- Status Bar -->
          <div class="md-statusbar">
            <div style="display:flex; align-items:center; gap:14px;">
              <span style="font-weight:700; color:var(--accent-color);">MARKDOWN</span>
              <span id="md-stat-words">0 ${isId ? 'kata' : 'words'}</span>
              <span id="md-stat-chars">0 ${isId ? 'karakter' : 'chars'}</span>
              <span id="md-stat-lines">1 ${isId ? 'baris' : 'lines'}</span>
            </div>
            <div id="md-stat-readtime">~1 ${isId ? 'mnt baca' : 'min read'}</div>
          </div>
        </div>
      </div>
    `;

    this.updateStats(currentContent);
  },

  /* ------------------------------------------------------------------------
     Toolbar & Writing Helpers
     ------------------------------------------------------------------------ */
  setViewMode(mode) {
    this.viewMode = mode;
    this.render();
  },

  toggleSidebar() {
    this.showSidebar = !this.showSidebar;
    this.render();
  },

  getEditor() {
    let el = document.getElementById('note-content-field');
    if (!el && this.viewMode === 'preview') {
      this.viewMode = 'split';
      this.render();
      el = document.getElementById('note-content-field');
    }
    return el;
  },

  wrapSelection(before, after, placeholder) {
    const ta = this.getEditor();
    if (!ta) return;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const selected = ta.value.substring(start, end) || placeholder;
    const replacement = `${before}${selected}${after}`;

    ta.setRangeText(replacement, start, end, 'select');
    ta.focus();
    ta.setSelectionRange(start + before.length, start + before.length + selected.length);
    this.onNoteChange();
  },

  insertHeading(level) {
    const prefix = '#'.repeat(level) + ' ';
    this.insertLinePrefix(prefix);
  },

  insertLinePrefix(prefix) {
    const ta = this.getEditor();
    if (!ta) return;
    const start = ta.selectionStart;
    const lineStart = ta.value.lastIndexOf('\n', start - 1) + 1;
    ta.setRangeText(prefix, lineStart, lineStart, 'end');
    ta.focus();
    this.onNoteChange();
  },

  insertSnippet(snippet) {
    const ta = this.getEditor();
    if (!ta) return;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    ta.setRangeText(snippet, start, end, 'end');
    ta.focus();
    this.onNoteChange();
  },

  insertCodeBlock() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const lang = prompt(isId ? 'Bahasa pemrograman (misal: rust, js, bash, python):' : 'Code language (e.g. rust, js, bash, python):', 'js');
    if (lang === null) return;
    this.insertSnippet(`\n\`\`\`${lang.trim()}\n// ${isId ? 'Tulis kode Anda di sini' : 'Write your code here'}\n\`\`\`\n`);
  },

  insertTable() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const tableMd = isId
      ? `\n| Kolom 1 | Kolom 2 | Kolom 3 |\n| --- | --- | --- |\n| Data A1 | Data B1 | Data C1 |\n| Data A2 | Data B2 | Data C2 |\n`
      : `\n| Column 1 | Column 2 | Column 3 |\n| --- | --- | --- |\n| Data A1 | Data B1 | Data C1 |\n| Data A2 | Data B2 | Data C2 |\n`;
    this.insertSnippet(tableMd);
  },

  insertLink() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const url = prompt(isId ? 'Masukkan URL tautan:' : 'Enter link URL:', 'https://');
    if (!url) return;
    this.wrapSelection('[', `](${url})`, isId ? 'Teks Tautan' : 'Link Text');
  },

  insertImage() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const url = prompt(isId ? 'Masukkan URL gambar:' : 'Enter image URL:', 'https://');
    if (!url) return;
    this.insertSnippet(`\n![${isId ? 'Deskripsi Gambar' : 'Image Alt'}](${url})\n`);
  },

  insertCallout() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    this.insertSnippet(isId
      ? `\n> 💡 **Catatan Penting:** Tuliskan informasi atau tips utama di sini.\n`
      : `\n> 💡 **Important Note:** Write key information or tips here.\n`);
  },

  handleEditorKeydown(e) {
    // Support Tab indentation & Ctrl+B / Ctrl+I / Ctrl+S shortcuts
    if (e.key === 'Tab') {
      e.preventDefault();
      this.insertSnippet('  ');
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
      e.preventDefault();
      this.wrapSelection('**', '**', 'bold');
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'i') {
      e.preventDefault();
      this.wrapSelection('*', '*', 'italic');
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (this.activeFile) this.saveFinderFile();
      else {
        const note = this.notes.find(n => n.id === this.activeNoteId);
        if (note) this.saveToBackend(note);
      }
    }
  },

  syncScroll(editorEl) {
    const previewEl = document.getElementById('md-live-preview');
    if (!previewEl || !editorEl) return;
    const maxEditorScroll = editorEl.scrollHeight - editorEl.clientHeight;
    if (maxEditorScroll <= 0) return;
    const ratio = editorEl.scrollTop / maxEditorScroll;
    previewEl.scrollTop = ratio * (previewEl.scrollHeight - previewEl.clientHeight);
  },

  /* ------------------------------------------------------------------------
     Native High-Fidelity Markdown Parser (Zero Dependencies)
     ------------------------------------------------------------------------ */
  escapeHtml(str) {
    return String(str || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  },

  escapeAttr(str) {
    return this.escapeHtml(str).replace(/"/g, '&quot;');
  },

  parseMarkdown(md) {
    if (!md || !md.trim()) {
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');
      return `<div style="color:var(--text-muted); font-style:italic; padding:20px 0;">${isId ? 'Pratinjau otomatis akan muncul secara langsung saat Anda mengetik...' : 'Live auto-preview will appear here as you type...'}</div>`;
    }

    // Extract fenced code blocks first so their contents aren't parsed as markdown
    const codeBlocks = [];
    let text = md.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, (_, lang, code) => {
      const idx = codeBlocks.length;
      const langBadge = lang ? `<div style="font-size:10px; font-weight:700; color:var(--text-muted); text-transform:uppercase; margin-bottom:6px;">${this.escapeHtml(lang)}</div>` : '';
      codeBlocks.push(`<pre>${langBadge}<code>${this.escapeHtml(code.replace(/\n$/, ''))}</code></pre>`);
      return `@@CODEBLOCK_${idx}@@`;
    });

    const lines = text.split(/\r?\n/);
    const out = [];
    let inUl = false;
    let inOl = false;
    let inTable = false;
    let tableHeaderDone = false;

    const closeLists = () => {
      if (inUl) { out.push('</ul>'); inUl = false; }
      if (inOl) { out.push('</ol>'); inOl = false; }
    };

    const closeTable = () => {
      if (inTable) { out.push('</tbody></table>'); inTable = false; tableHeaderDone = false; }
    };

    const formatInline = (line) => {
      let s = this.escapeHtml(line);
      // Inline code
      s = s.replace(/`([^`]+)`/g, '<code>$1</code>');
      // Images
      s = s.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '<img src="$2" alt="$1">');
      // Links
      s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>');
      // Bold + Italic
      s = s.replace(/\*\*\*([^*]+)\*\*\*/g, '<strong><em>$1</em></strong>');
      // Bold
      s = s.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
      // Italic
      s = s.replace(/\*([^*]+)\*/g, '<em>$1</em>');
      // Strikethrough
      s = s.replace(/~~([^~]+)~~/g, '<del>$1</del>');
      // Highlight
      s = s.replace(/==([^=]+)==/g, '<mark>$1</mark>');
      return s;
    };

    for (let i = 0; i < lines.length; i++) {
      const raw = lines[i];
      const trimmed = raw.trim();

      // Code block placeholder
      const cbMatch = trimmed.match(/^@@CODEBLOCK_(\d+)@@$/);
      if (cbMatch) {
        closeLists();
        closeTable();
        out.push(codeBlocks[parseInt(cbMatch[1])]);
        continue;
      }

      // Empty line
      if (!trimmed) {
        closeLists();
        closeTable();
        continue;
      }

      // Horizontal rule
      if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
        closeLists();
        closeTable();
        out.push('<hr>');
        continue;
      }

      // Headings
      const hMatch = trimmed.match(/^(#{1,6})\s+(.+)$/);
      if (hMatch) {
        closeLists();
        closeTable();
        const lvl = hMatch[1].length;
        out.push(`<h${lvl}>${formatInline(hMatch[2])}</h${lvl}>`);
        continue;
      }

      // Table row
      if (trimmed.startsWith('|') && trimmed.endsWith('|')) {
        closeLists();
        const cells = trimmed.slice(1, -1).split('|').map(c => c.trim());
        const isSeparator = cells.every(c => /^:?-{2,}:?$/.test(c));
        if (!inTable) {
          inTable = true;
          tableHeaderDone = false;
          out.push('<table><thead><tr>' + cells.map(c => `<th>${formatInline(c)}</th>`).join('') + '</tr></thead><tbody>');
          continue;
        }
        if (isSeparator && !tableHeaderDone) {
          tableHeaderDone = true;
          continue;
        }
        out.push('<tr>' + cells.map(c => `<td>${formatInline(c)}</td>`).join('') + '</tr>');
        continue;
      } else {
        closeTable();
      }

      // Blockquote
      if (trimmed.startsWith('>')) {
        closeLists();
        out.push(`<blockquote>${formatInline(trimmed.replace(/^>\s?/, ''))}</blockquote>`);
        continue;
      }

      // Task list item (- [ ] or - [x])
      const taskMatch = trimmed.match(/^[-*]\s+\[([ xX])\]\s+(.+)$/);
      if (taskMatch) {
        if (inOl) { out.push('</ol>'); inOl = false; }
        if (!inUl) { out.push('<ul>'); inUl = true; }
        const checked = taskMatch[1].toLowerCase() === 'x';
        out.push(`<li class="md-task-item"><input type="checkbox" ${checked ? 'checked' : ''} disabled style="accent-color:var(--accent-color);"><span style="${checked ? 'text-decoration:line-through; opacity:0.65;' : ''}">${formatInline(taskMatch[2])}</span></li>`);
        continue;
      }

      // Unordered list
      const ulMatch = trimmed.match(/^[-*+]\s+(.+)$/);
      if (ulMatch) {
        if (inOl) { out.push('</ol>'); inOl = false; }
        if (!inUl) { out.push('<ul>'); inUl = true; }
        out.push(`<li>${formatInline(ulMatch[1])}</li>`);
        continue;
      }

      // Ordered list
      const olMatch = trimmed.match(/^\d+\.\s+(.+)$/);
      if (olMatch) {
        if (inUl) { out.push('</ul>'); inUl = false; }
        if (!inOl) { out.push('<ol>'); inOl = true; }
        out.push(`<li>${formatInline(olMatch[1])}</li>`);
        continue;
      }

      // Normal paragraph
      closeLists();
      out.push(`<p>${formatInline(trimmed)}</p>`);
    }

    closeLists();
    closeTable();
    return out.join('\n');
  },

  /* ------------------------------------------------------------------------
     State Updates, Stats & Persistence
     ------------------------------------------------------------------------ */
  updateStats(content) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const text = content || '';
    const words = text.trim() ? text.trim().split(/\s+/).length : 0;
    const chars = text.length;
    const lines = text ? text.split(/\r?\n/).length : 1;
    const mins = Math.max(1, Math.ceil(words / 200));

    const wEl = document.getElementById('md-stat-words');
    const cEl = document.getElementById('md-stat-chars');
    const lEl = document.getElementById('md-stat-lines');
    const rEl = document.getElementById('md-stat-readtime');
    if (wEl) wEl.textContent = `${words} ${isId ? 'kata' : 'words'}`;
    if (cEl) cEl.textContent = `${chars} ${isId ? 'karakter' : 'chars'}`;
    if (lEl) lEl.textContent = `${lines} ${isId ? 'baris' : 'lines'}`;
    if (rEl) rEl.textContent = `~${mins} ${isId ? 'mnt baca' : 'min read'}`;
  },

  selectNote(id) {
    this.activeFile = null;
    this.activeNoteId = id;
    this.isDirty = false;
    this.render();
    this.updateWindowTitle();
  },

  createNote() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    this.activeFile = null;
    const newNote = {
      id: `note_${Date.now()}`,
      title: isId ? 'Dokumen Markdown Baru' : 'New Markdown Document',
      content: isId
        ? `# Dokumen Markdown Baru\n\nMulai menulis dengan **Auto Preview** langsung di sebelah kanan.\n\n- [x] Mendukung format Markdown lengkap\n- [ ] Tambahkan catatan atau tabel di sini\n`
        : `# New Markdown Document\n\nStart writing with live **Auto Preview** on the right.\n\n- [x] Full Markdown formatting support\n- [ ] Add your notes or tables here\n`,
      tags: 'markdown',
      pinned: false,
      updated_at: isId ? 'Baru saja' : 'Just now'
    };
    this.notes.unshift(newNote);
    this.activeNoteId = newNote.id;
    this.render();
    this.updateWindowTitle();
    this.saveToBackend(newNote);
  },

  onNoteChange() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const titleEl = document.getElementById('note-title-field');
    const contentEl = document.getElementById('note-content-field');
    if (!contentEl) return;

    const content = contentEl.value;
    const title = titleEl ? titleEl.value : '';

    // 1. Instant Live Auto Preview update
    const previewEl = document.getElementById('md-live-preview');
    if (previewEl) {
      previewEl.innerHTML = this.parseMarkdown(content);
    }

    // 2. Live status bar statistics
    this.updateStats(content);

    // 3. Persist state
    const statusEl = document.getElementById('note-save-status');
    if (this.activeFile) {
      this.fileContent = content;
      this.isDirty = true;
      this.updateWindowTitle();
      if (statusEl) statusEl.textContent = isId ? '● Belum disimpan' : '● Unsaved';
      if (this._saveTimeout) clearTimeout(this._saveTimeout);
      this._saveTimeout = setTimeout(() => this.saveFinderFile(), 1000);
      return;
    }

    const note = this.notes.find(n => n.id === this.activeNoteId);
    if (note) {
      note.title = title;
      note.content = content;
      if (statusEl) statusEl.textContent = isId ? 'Menyimpan...' : 'Saving...';

      if (this._saveTimeout) clearTimeout(this._saveTimeout);
      this._saveTimeout = setTimeout(() => {
        this.saveToBackend(note);
      }, 600);
    }
  },

  async saveToBackend(note) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    try {
      await fetch('/api/notes', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...this.getAuthHeaders() },
        body: JSON.stringify(note)
      });
      const statusEl = document.getElementById('note-save-status');
      if (statusEl) statusEl.textContent = isId ? '● Tersimpan' : '● Saved';
    } catch {}
  },

  async saveFinderFile() {
    if (!this.activeFile) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    try {
      const res = await fetch(`/api/files/${this.activeFile.id}/content`, {
        method: 'PUT',
        headers: { 'Content-Type': 'text/plain; charset=utf-8', ...this.getAuthHeaders() },
        body: this.fileContent
      });
      if (res.ok) {
        this.isDirty = false;
        this.updateWindowTitle();
        const statusEl = document.getElementById('note-save-status');
        if (statusEl) statusEl.textContent = isId ? '● Tersimpan di Finder' : '● Saved to Finder';
      }
    } catch (e) {
      console.warn('Failed to save markdown file:', e);
    }
  },

  async saveAsFinderMd() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const rawTitle = (this.getActiveTitle() || 'Dokumentasi').replace(/\.md$/i, '');
    const fileNameInput = prompt(isId ? 'Simpan dokumen Markdown ke Finder (folder Documents):' : 'Save Markdown document to Finder (Documents folder):', `${rawTitle}.md`);
    if (!fileNameInput || !fileNameInput.trim()) return;

    const finalName = fileNameInput.trim().endsWith('.md') ? fileNameInput.trim() : `${fileNameInput.trim()}.md`;
    const content = this.getActiveContent();

    try {
      const res = await fetch('/api/files', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...this.getAuthHeaders() },
        body: JSON.stringify({
          name: finalName,
          parent_id: 'fld_docs',
          is_folder: false,
          extension: 'md',
          content
        })
      });
      if (res.ok) {
        const created = await res.json();
        if (window.DokuIsland) {
          DokuIsland.success(
            isId ? `Dokumen "${finalName}" tersimpan di Finder!` : `Document "${finalName}" saved to Finder!`,
            'Markdown Studio'
          );
        }
        if (window.DokuFinder) window.DokuFinder.loadAllFiles();
        if (created && created.id) {
          await this.openFinderFile(created);
        }
      }
    } catch (e) {
      alert('Error: ' + e);
    }
  },

  downloadMdFile() {
    const title = (this.getActiveTitle() || 'document').replace(/\.md$/i, '');
    const content = this.getActiveContent();
    const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = `${title}.md`;
    a.click();
    URL.revokeObjectURL(a.href);
  },

  printPreview() {
    const html = this.parseMarkdown(this.getActiveContent());
    const title = this.escapeHtml(this.getActiveTitle());
    const win = window.open('', '_blank', 'width=840,height=700');
    if (!win) return;
    win.document.write(`
      <!DOCTYPE html>
      <html>
      <head>
        <title>${title}</title>
        <style>
          body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; max-width: 780px; margin: 40px auto; padding: 0 24px; color: #1d1d1f; line-height: 1.7; }
          h1, h2 { border-bottom: 1px solid #e5e7eb; padding-bottom: 6px; }
          pre { background: #f5f5f7; padding: 12px; border-radius: 8px; overflow-x: auto; }
          code { background: #f3f4f6; padding: 2px 5px; border-radius: 4px; font-family: monospace; }
          pre code { background: none; padding: 0; }
          blockquote { border-left: 4px solid #007aff; padding: 6px 14px; background: #f0f7ff; margin: 12px 0; }
          table { width: 100%; border-collapse: collapse; margin: 14px 0; }
          th, td { border: 1px solid #d1d5db; padding: 7px 10px; text-align: left; }
          th { background: #f9fafb; }
        </style>
      </head>
      <body>${html}</body>
      </html>
    `);
    win.document.close();
    win.focus();
    setTimeout(() => win.print(), 250);
  },

  togglePin() {
    const note = this.notes.find(n => n.id === this.activeNoteId);
    if (note) {
      note.pinned = !note.pinned;
      this.saveToBackend(note);
      this.render();
    }
  },

  async deleteActiveNote() {
    if (!this.activeNoteId) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (!confirm(isId ? "Hapus dokumen ini?" : "Delete this document?")) return;

    try {
      await fetch(`/api/notes/${this.activeNoteId}`, {
        method: 'DELETE',
        headers: this.getAuthHeaders()
      });
      this.notes = this.notes.filter(n => n.id !== this.activeNoteId);
      this.activeNoteId = this.notes.length > 0 ? this.notes[0].id : null;
      this.render();
    } catch (e) {
      alert("Gagal menghapus: " + e);
    }
  }
};
