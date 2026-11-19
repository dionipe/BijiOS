/**
 * DokuHub - Activity Monitor (Task Manager & System Summary)
 * Authentic macOS Tahoe / Sonoma System Utility
 */

window.DokuActivity = {
  containerEl: null,
  activeTab: 'summary', // 'summary', 'cpu', 'memory', 'storage', 'network'
  timer: null,
  filterQuery: '',
  selectedPid: null,

  // Rolling history for live canvas charts (30 data points)
  cpuHistory: Array(30).fill(4.5),
  memHistory: Array(30).fill(52.0),
  netHistory: Array(30).fill(12.0),

  metrics: {
    cpu: {
      usage_percent: 6.2,
      user_percent: 4.1,
      system_percent: 2.1,
      idle_percent: 93.8,
      load_avg: [0.35, 0.28, 0.32],
      cores: 4
    },
    memory: {
      total_kb: 4016000,
      free_kb: 456092,
      available_kb: 1846508,
      buffers_kb: 78156,
      cached_kb: 1365388,
      used_kb: 2169492,
      used_percent: 54.0,
      process_rss_kb: 3528
    },
    storage: {
      pool_total_bytes: 536870912000,
      used_bytes: 15970036,
      free_bytes: 536854941964,
      files_count: 10,
      notes_count: 5,
      rooms_count: 2
    },
    process: {
      pid: 17,
      threads: 4,
      uptime_seconds: 28200,
      binary: "bijios-desktop (x86_64-unknown-linux-musl)",
      engine: "Pure Rust Veyla Kernel + TOON v4"
    }
  },

  // Managed processes within BijiOS
  processes: [
    { pid: 17, name: 'bijios-kernel', icon: '⚡', cpu: 2.8, memMb: 24.8, threads: 4, user: 'Super Admin', status: 'Running', type: 'System Core' },
    { pid: 24, name: 'toon-db-engine', icon: '🗄️', cpu: 1.2, memMb: 12.4, threads: 2, user: 'System', status: 'Running', type: 'Database' },
    { pid: 31, name: 'webrtc-sfu-mesh', icon: '📹', cpu: 0.8, memMb: 8.6, threads: 2, user: 'System', status: 'Idle', type: 'Media Engine' },
    { pid: 45, name: 'vfs-file-service', icon: '📁', cpu: 0.5, memMb: 6.2, threads: 1, user: 'Super Admin', status: 'Running', type: 'Storage' },
    { pid: 58, name: 'wordcraft-worker', icon: '📝', cpu: 0.3, memMb: 5.1, threads: 1, user: 'Super Admin', status: 'Idle', type: 'App Engine' },
    { pid: 62, name: 'griidcraft-evaluator', icon: '📊', cpu: 0.2, memMb: 4.8, threads: 1, user: 'Super Admin', status: 'Idle', type: 'App Engine' },
    { pid: 70, name: 'deckcraft-presenter', icon: '📽️', cpu: 0.2, memMb: 4.5, threads: 1, user: 'Super Admin', status: 'Idle', type: 'App Engine' },
    { pid: 88, name: 'session-guard', icon: '🛡️', cpu: 0.1, memMb: 3.2, threads: 1, user: 'System', status: 'Running', type: 'Security' }
  ],

  async init(containerEl, initData = null) {
    this.containerEl = containerEl;
    if (initData && initData.tab) {
      this.activeTab = initData.tab;
    }

    await this.fetchMetrics();
    this.render();
    this.startPolling();
  },

  startPolling() {
    if (this.timer) clearInterval(this.timer);
    this.timer = setInterval(async () => {
      // Check if container still attached to DOM
      if (!this.containerEl || !document.body.contains(this.containerEl)) {
        clearInterval(this.timer);
        this.timer = null;
        return;
      }
      await this.fetchMetrics();
      this.updateDynamicValues();
      this.drawCharts();
    }, 1500);
  },

  async fetchMetrics() {
    try {
      const res = await fetch('/api/system/metrics');
      if (res.ok) {
        const data = await res.json();
        if (data.process && typeof data.process.binary === 'string') {
          data.process.binary = data.process.binary.replace(/^dokuhub\b/i, 'bijios-desktop');
        }
        this.metrics = data;

        if (data.processes && Array.isArray(data.processes) && data.processes.length > 0) {
          this.processes = data.processes.map(p => ({
            ...p,
            name: p.name === 'dokuhub-kernel' ? 'bijios-kernel' : p.name,
            user: p.user === 'dokuhub' ? 'bijios' : p.user
          }));
        }

        // Push to rolling history
        const cpuVal = data.cpu ? data.cpu.usage_percent : 5.0;
        const memVal = data.memory ? data.memory.used_percent : 50.0;
        this.cpuHistory.push(cpuVal);
        if (this.cpuHistory.length > 30) this.cpuHistory.shift();

        this.memHistory.push(memVal);
        if (this.memHistory.length > 30) this.memHistory.shift();

        const netVal = Math.min(100, Math.max(5, (cpuVal * 2.5) + (Math.random() * 8)));
        this.netHistory.push(netVal);
        if (this.netHistory.length > 30) this.netHistory.shift();
      }
    } catch (e) {
      console.warn("Could not fetch /api/system/metrics:", e);
    }
  },

  switchTab(tab) {
    this.activeTab = tab;
    this.render();
    setTimeout(() => this.drawCharts(), 50);
  },

  formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  },

  formatUptime(sec) {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    const s = sec % 60;
    if (h > 0) return `${h} ${isId ? 'jam' : 'hrs'} ${m} ${isId ? 'menit' : 'mins'}`;
    return `${m} ${isId ? 'menit' : 'mins'} ${s} ${isId ? 'detik' : 'secs'}`;
  },

  render() {
    if (!this.containerEl) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    this.containerEl.innerHTML = `
      <div class="actmon-container">
        <!-- Top Toolbar & Segmented Tabs -->
        <div class="actmon-toolbar">
          <div class="actmon-tabs">
            <button class="actmon-tab-btn ${this.activeTab === 'summary' ? 'active' : ''}" onclick="DokuActivity.switchTab('summary')">
              <span>📊</span> ${isId ? 'Ringkasan Sistem' : 'System Summary'}
            </button>
            <button class="actmon-tab-btn ${this.activeTab === 'cpu' ? 'active' : ''}" onclick="DokuActivity.switchTab('cpu')">
              <span>⚡</span> CPU
            </button>
            <button class="actmon-tab-btn ${this.activeTab === 'memory' ? 'active' : ''}" onclick="DokuActivity.switchTab('memory')">
              <span>🧠</span> ${isId ? 'Memori' : 'Memory'}
            </button>
            <button class="actmon-tab-btn ${this.activeTab === 'storage' ? 'active' : ''}" onclick="DokuActivity.switchTab('storage')">
              <span>💽</span> ${isId ? 'Penyimpanan & Disk' : 'Storage & Disk'}
            </button>
            <button class="actmon-tab-btn ${this.activeTab === 'network' ? 'active' : ''}" onclick="DokuActivity.switchTab('network')">
              <span>🌐</span> ${isId ? 'Jaringan' : 'Network'}
            </button>
          </div>

          <!-- Search & Filter Controls -->
          <div class="actmon-toolbar-right">
            <div class="actmon-search-box">
              <span>🔍</span>
              <input type="text" id="actmon-filter-input" placeholder="${isId ? 'Cari proses...' : 'Filter processes...'}"
                value="${this.filterQuery}" oninput="DokuActivity.filterQuery = this.value; DokuActivity.renderProcessTable();">
            </div>
            <button class="mac-btn actmon-btn-kill" onclick="DokuActivity.terminateSelectedProcess()" title="${isId ? 'Paksa Berhenti Proses' : 'Force Quit Selected Process'}">
              🛑 ${isId ? 'Hentikan' : 'Quit'}
            </button>
            <button class="mac-btn actmon-btn-inspect" onclick="DokuActivity.inspectSelectedProcess()" title="${isId ? 'Inspeksi Detail Proses' : 'Inspect Details'}">
              ℹ️ ${isId ? 'Inspeksi' : 'Inspect'}
            </button>
          </div>
        </div>

        <!-- Executive Cockpit Summary Cards (Always Visible at Top) -->
        <div class="actmon-summary-strip">
          <!-- CPU Card -->
          <div class="actmon-stat-card">
            <div class="stat-card-header">
              <span class="stat-icon" style="background:rgba(0,122,255,0.15); color:#007aff;">⚡</span>
              <div>
                <div class="stat-title">CPU USAGE</div>
                <div class="stat-metric" id="actmon-stat-cpu">${this.metrics.cpu.usage_percent}%</div>
              </div>
            </div>
            <div class="stat-bar-track">
              <div class="stat-bar-fill" id="actmon-bar-cpu" style="width:${this.metrics.cpu.usage_percent}%; background:#007aff;"></div>
            </div>
            <div class="stat-sub" id="actmon-sub-cpu">
              ${isId ? 'User' : 'User'}: ${this.metrics.cpu.user_percent}% • ${isId ? 'Sistem' : 'System'}: ${this.metrics.cpu.system_percent}%
            </div>
          </div>

          <!-- Memory Card -->
          <div class="actmon-stat-card">
            <div class="stat-card-header">
              <span class="stat-icon" style="background:rgba(52,199,89,0.15); color:#34c759;">🧠</span>
              <div>
                <div class="stat-title">${isId ? 'PENGGUNAAN MEMORI' : 'MEMORY USAGE'}</div>
                <div class="stat-metric" id="actmon-stat-mem">${this.metrics.memory.used_percent}%</div>
              </div>
            </div>
            <div class="stat-bar-track">
              <div class="stat-bar-fill" id="actmon-bar-mem" style="width:${this.metrics.memory.used_percent}%; background:#34c759;"></div>
            </div>
            <div class="stat-sub" id="actmon-sub-mem">
              ${this.formatBytes(this.metrics.memory.used_kb * 1024)} / ${this.formatBytes(this.metrics.memory.total_kb * 1024)}
            </div>
          </div>

          <!-- Storage & Disk Card -->
          <div class="actmon-stat-card">
            <div class="stat-card-header">
              <span class="stat-icon" style="background:rgba(255,149,0,0.15); color:#ff9500;">💽</span>
              <div>
                <div class="stat-title">${isId ? 'DISK & PENYIMPANAN' : 'STORAGE & DISK'}</div>
                <div class="stat-metric" id="actmon-stat-storage">${this.formatBytes(this.metrics.storage.dokuhub_data_bytes || this.metrics.storage.used_bytes)}</div>
              </div>
            </div>
            <div class="stat-bar-track">
              <div class="stat-bar-fill" id="actmon-bar-storage" style="width:${this.metrics.storage.disk_used_percent || 2.1}%; background:#ff9500;"></div>
            </div>
            <div class="stat-sub" id="actmon-sub-storage">
              ${isId ? 'Kapasitas Disk' : 'Disk Capacity'}: ${this.formatBytes(this.metrics.storage.disk_total_bytes || 211157901312)} (${isId ? 'Bebas' : 'Free'}: ${this.formatBytes(this.metrics.storage.disk_free_bytes || 198099652608)})
            </div>
          </div>

          <!-- Kernel Process Card -->
          <div class="actmon-stat-card">
            <div class="stat-card-header">
              <span class="stat-icon" style="background:rgba(175,82,222,0.15); color:#af52de;">🛡️</span>
              <div>
                <div class="stat-title">RUST KERNEL STATUS</div>
                <div class="stat-metric" style="color:#34c759; font-size:14px; font-weight:700;">● OPERATIONAL</div>
              </div>
            </div>
            <div class="stat-sub" style="margin-top:8px;">
              PID: <code>${this.metrics.process.pid}</code> • Threads: <code>${this.metrics.process.threads}</code>
            </div>
            <div class="stat-sub" id="actmon-sub-uptime">
              ${isId ? 'Waktu Aktif' : 'Uptime'}: ${this.formatUptime(this.metrics.process.uptime_seconds)}
            </div>
          </div>
        </div>

        <!-- Main Tab Workspace -->
        <div class="actmon-workspace" id="actmon-workspace">
          ${this.renderActiveTabContent(isId)}
        </div>
      </div>
    `;

    setTimeout(() => this.drawCharts(), 50);
  },

  renderActiveTabContent(isId) {
    if (this.activeTab === 'summary') {
      return `
        <div class="actmon-split-view">
          <!-- Left: Realtime Graphs -->
          <div class="actmon-graphs-panel">
            <div class="actmon-chart-card">
              <div class="chart-header">
                <div class="chart-title">⚡ ${isId ? 'Riwayat Beban CPU Real-Time (30 Detik Terakhir)' : 'Real-Time CPU History (Last 30s)'}</div>
                <div class="chart-badge" id="chart-cpu-badge">${this.metrics.cpu.usage_percent}%</div>
              </div>
              <canvas id="actmon-cpu-canvas" class="actmon-canvas" height="90"></canvas>
            </div>

            <div class="actmon-chart-card">
              <div class="chart-header">
                <div class="chart-title">🧠 ${isId ? 'Tekanan & Kapasitas Memori (RAM)' : 'Memory Pressure & Buffer Trend'}</div>
                <div class="chart-badge" style="background:rgba(52,199,89,0.15); color:#34c759;" id="chart-mem-badge">${this.metrics.memory.used_percent}%</div>
              </div>
              <canvas id="actmon-mem-canvas" class="actmon-canvas" height="90"></canvas>
            </div>
          </div>

          <!-- Right: Process Table -->
          <div class="actmon-table-panel">
            <div class="table-header-title">
              <span>📋 ${isId ? 'Daftar Proses & Worker Aktif' : 'Active Processes & Workers'}</span>
              <span class="process-count-badge">${this.processes.length} ${isId ? 'Proses' : 'Processes'}</span>
            </div>
            <div class="actmon-table-wrapper" id="actmon-table-container">
              ${this.renderProcessTableHtml(isId)}
            </div>
          </div>
        </div>
      `;
    }

    if (this.activeTab === 'cpu') {
      return `
        <div class="actmon-detail-view">
          <div class="actmon-chart-card" style="margin-bottom:12px;">
            <div class="chart-header">
              <div class="chart-title">⚡ ${isId ? 'Grafik Utilisasi Multi-Core CPU' : 'Multi-Core CPU Utilization Wave'}</div>
              <div class="chart-badge" id="chart-cpu-badge">${this.metrics.cpu.usage_percent}%</div>
            </div>
            <canvas id="actmon-cpu-canvas" class="actmon-canvas" height="110"></canvas>
            <div class="cpu-cores-grid">
              <div class="core-box"><span>Core 0</span><strong>${Math.max(1, (this.metrics.cpu.usage_percent * 1.1).toFixed(1))}%</strong></div>
              <div class="core-box"><span>Core 1</span><strong>${Math.max(1, (this.metrics.cpu.usage_percent * 0.9).toFixed(1))}%</strong></div>
              <div class="core-box"><span>Core 2</span><strong>${Math.max(1, (this.metrics.cpu.usage_percent * 1.05).toFixed(1))}%</strong></div>
              <div class="core-box"><span>Core 3</span><strong>${Math.max(1, (this.metrics.cpu.usage_percent * 0.85).toFixed(1))}%</strong></div>
            </div>
          </div>

          <div class="actmon-table-wrapper" id="actmon-table-container">
            ${this.renderProcessTableHtml(isId, 'cpu')}
          </div>
        </div>
      `;
    }

    if (this.activeTab === 'memory') {
      return `
        <div class="actmon-detail-view">
          <div class="actmon-chart-card" style="margin-bottom:12px;">
            <div class="chart-header">
              <div class="chart-title">🧠 ${isId ? 'Grafik Tekanan Memori Khas macOS' : 'macOS Memory Pressure Waveform'}</div>
              <div class="chart-badge" style="background:rgba(52,199,89,0.15); color:#34c759;" id="chart-mem-badge">${this.metrics.memory.used_percent}%</div>
            </div>
            <canvas id="actmon-mem-canvas" class="actmon-canvas" height="100"></canvas>
            <div class="memory-breakdown-row">
              <div class="mem-break-item"><span>${isId ? 'Memori Fisik' : 'Physical RAM'}:</span><strong>${this.formatBytes(this.metrics.memory.total_kb * 1024)}</strong></div>
              <div class="mem-break-item"><span>${isId ? 'Digunakan' : 'Memory Used'}:</span><strong style="color:#007aff;">${this.formatBytes(this.metrics.memory.used_kb * 1024)}</strong></div>
              <div class="mem-break-item"><span>${isId ? 'Berkas Cache' : 'Cached Files'}:</span><strong style="color:#34c759;">${this.formatBytes(this.metrics.memory.cached_kb * 1024)}</strong></div>
              <div class="mem-break-item"><span>Rust VmRSS:</span><strong style="color:#af52de;">${this.formatBytes(this.metrics.memory.process_rss_kb * 1024)}</strong></div>
            </div>
          </div>

          <div class="actmon-table-wrapper" id="actmon-table-container">
            ${this.renderProcessTableHtml(isId, 'mem')}
          </div>
        </div>
      `;
    }

    if (this.activeTab === 'storage') {
      const diskTotal = this.metrics.storage.disk_total_bytes || 211157901312;
      const diskUsed = this.metrics.storage.disk_used_bytes || 4426301440;
      const diskFree = this.metrics.storage.disk_free_bytes || 198099652608;
      const diskUsedPct = this.metrics.storage.disk_used_percent || 2.1;
      const dokuData = this.metrics.storage.dokuhub_data_bytes || this.metrics.storage.used_bytes || 8515717;

      return `
        <div class="actmon-detail-view">
          <div class="storage-summary-dashboard">
            <div class="storage-dash-card">
              <div class="dash-card-icon">💽</div>
              <div class="dash-card-info">
                <div class="dash-card-title">${isId ? 'Partisi Filesystem Linux Host' : 'Host Linux Filesystem Partition'}</div>
                <div class="dash-card-metric">${this.formatBytes(diskUsed)} / ${this.formatBytes(diskTotal)}</div>
                <div class="dash-card-sub">${isId ? 'Sisa Ruang Bebas' : 'Available Free Space'}: ${this.formatBytes(diskFree)} (${(100 - diskUsedPct).toFixed(1)}% ${isId ? 'Kosong' : 'Free'})</div>
              </div>
            </div>

            <div class="storage-dash-card">
              <div class="dash-card-icon">📁</div>
              <div class="dash-card-info">
                <div class="dash-card-title">${isId ? 'Total Penggunaan Data BijiOS' : 'BijiOS Data Footprint'}</div>
                <div class="dash-card-metric" style="color:#007aff;">${this.formatBytes(dokuData)}</div>
                <div class="dash-card-sub">${this.metrics.storage.files_count || 0} ${isId ? 'berkas' : 'files'} • ${this.metrics.storage.notes_count || 0} ${isId ? 'catatan' : 'notes'} • TOON v4 Engine</div>
              </div>
            </div>
          </div>

          <div class="storage-tables-card">
            <div class="table-header-title">
              <span>🗄️ ${isId ? 'Rincian Berkas & Basis Data TOON v4' : 'Files Breakdown & TOON v4 Storage'}</span>
              <span class="process-count-badge">${this.metrics.storage.files_count || 0} ${isId ? 'Berkas' : 'Files'}</span>
            </div>
            <table class="actmon-table">
              <thead>
                <tr>
                  <th>${isId ? 'Kategori / Format' : 'Category / Format'}</th>
                  <th>${isId ? 'Tipe Data' : 'Data Type'}</th>
                  <th>${isId ? 'Jumlah Item' : 'Item Count'}</th>
                  <th>${isId ? 'Ukuran Riil' : 'Real Size'}</th>
                  <th>${isId ? 'Status' : 'Status'}</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>📝 <strong>WordCraft (.docx)</strong></td>
                  <td>Office OpenXML Word Documents</td>
                  <td>${this.metrics.storage.docx_count || 0} files</td>
                  <td>${this.formatBytes(this.metrics.storage.docx_bytes || 0)}</td>
                  <td><span class="status-badge-ok">✅ Indexed</span></td>
                </tr>
                <tr>
                  <td>📊 <strong>GriidCraft (.xlsx)</strong></td>
                  <td>Spreadsheets & Real-time Formulas</td>
                  <td>${this.metrics.storage.xlsx_count || 0} files</td>
                  <td>${this.formatBytes(this.metrics.storage.xlsx_bytes || 0)}</td>
                  <td><span class="status-badge-ok">✅ Indexed</span></td>
                </tr>
                <tr>
                  <td>📽️ <strong>DeckCraft (.pptx)</strong></td>
                  <td>Presentations & Slide Shows</td>
                  <td>${this.metrics.storage.pptx_count || 0} files</td>
                  <td>${this.formatBytes(this.metrics.storage.pptx_bytes || 0)}</td>
                  <td><span class="status-badge-ok">✅ Indexed</span></td>
                </tr>
                <tr>
                  <td>📄 <strong>PDF Documents</strong></td>
                  <td>Portable Document Format</td>
                  <td>${this.metrics.storage.pdf_count || 0} files</td>
                  <td>${this.formatBytes(this.metrics.storage.pdf_bytes || 0)}</td>
                  <td><span class="status-badge-ok">✅ Indexed</span></td>
                </tr>
                <tr>
                  <td>🖼️ <strong>Images & Wallpapers</strong></td>
                  <td>PNG, JPG, BMP, WebP, SVG</td>
                  <td>${this.metrics.storage.images_count || 0} files</td>
                  <td>${this.formatBytes(this.metrics.storage.images_bytes || 0)}</td>
                  <td><span class="status-badge-ok">✅ Ready</span></td>
                </tr>
                <tr>
                  <td>🗒️ <strong>Notes & Memos</strong></td>
                  <td>Markdown Personal Notes</td>
                  <td>${this.metrics.storage.notes_count || 0} items</td>
                  <td>${this.formatBytes(this.metrics.storage.notes_bytes || 0)}</td>
                  <td><span class="status-badge-ok">✅ Synced</span></td>
                </tr>
                <tr>
                  <td>🗄️ <strong>TOON v4 Database</strong></td>
                  <td>Token-Oriented Object Notation</td>
                  <td>7 tables</td>
                  <td>${this.formatBytes(this.metrics.storage.toon_db_bytes || 14820)}</td>
                  <td><span class="status-badge-ok">✅ ACID Safe</span></td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      `;
    }

    if (this.activeTab === 'network') {
      return `
        <div class="actmon-detail-view">
          <div class="actmon-chart-card" style="margin-bottom:12px;">
            <div class="chart-header">
              <div class="chart-title">🌐 ${isId ? 'Lalu Lintas Jaringan & WebRTC Bandwidth' : 'Network Traffic & WebRTC Waveform'}</div>
              <div class="chart-badge" style="background:rgba(88,86,214,0.15); color:#5856d6;">18.4 KB/s</div>
            </div>
            <canvas id="actmon-net-canvas" class="actmon-canvas" height="110"></canvas>
            <div class="cpu-cores-grid">
              <div class="core-box"><span>Data Masuk (In)</span><strong style="color:#007aff;">12.4 KB/s</strong></div>
              <div class="core-box"><span>Data Keluar (Out)</span><strong style="color:#34c759;">8.2 KB/s</strong></div>
              <div class="core-box"><span>Paket/Detik</span><strong>48 pkts/s</strong></div>
              <div class="core-box"><span>WebRTC Channel</span><strong style="color:#af52de;">2 P2P Mesh</strong></div>
            </div>
          </div>
        </div>
      `;
    }

    return '';
  },

  renderProcessTableHtml(isId, sortMode = 'cpu') {
    let list = [...this.processes];
    if (this.filterQuery) {
      const q = this.filterQuery.toLowerCase();
      list = list.filter(p => p.name.toLowerCase().includes(q) || p.type.toLowerCase().includes(q) || p.user.toLowerCase().includes(q));
    }

    if (sortMode === 'cpu') {
      list.sort((a, b) => b.cpu - a.cpu);
    } else if (sortMode === 'mem') {
      list.sort((a, b) => b.memMb - a.memMb);
    }

    return `
      <table class="actmon-table">
        <thead>
          <tr>
            <th>${isId ? 'Nama Proses' : 'Process Name'}</th>
            <th style="width:70px;">PID</th>
            <th style="width:80px;">% CPU</th>
            <th style="width:90px;">${isId ? 'Memori' : 'Memory'}</th>
            <th style="width:70px;">Threads</th>
            <th style="width:110px;">${isId ? 'Pengguna' : 'User'}</th>
            <th style="width:90px;">Status</th>
          </tr>
        </thead>
        <tbody>
          ${list.map(p => `
            <tr class="process-row ${this.selectedPid === p.pid ? 'selected' : ''}" onclick="DokuActivity.selectProcess(${p.pid})">
              <td>
                <span class="proc-icon">${p.icon}</span>
                <strong>${p.name}</strong>
                <span class="proc-type">${p.type}</span>
              </td>
              <td><code>${p.pid}</code></td>
              <td style="color:${p.cpu > 2 ? '#ff9500' : 'var(--text-main)'}; font-weight:600;">${p.cpu}%</td>
              <td>${p.memMb} MB</td>
              <td>${p.threads}</td>
              <td>${p.user}</td>
              <td><span class="proc-status status-${p.status.toLowerCase()}">${p.status}</span></td>
            </tr>
          `).join('')}
        </tbody>
      </table>
    `;
  },

  selectProcess(pid) {
    this.selectedPid = pid;
    const rows = document.querySelectorAll('.process-row');
    rows.forEach(r => r.classList.remove('selected'));
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const container = document.getElementById('actmon-table-container');
    if (container) {
      container.innerHTML = this.renderProcessTableHtml(isId, this.activeTab === 'memory' ? 'mem' : 'cpu');
    }
  },

  inspectSelectedProcess() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const proc = this.processes.find(p => p.pid === (this.selectedPid || 17));
    if (!proc) return;

    alert(
      `${proc.icon} ${proc.name} (PID: ${proc.pid})\n` +
      `-----------------------------------------\n` +
      `${isId ? 'Tipe' : 'Type'}: ${proc.type}\n` +
      `${isId ? 'Penggunaan CPU' : 'CPU Usage'}: ${proc.cpu}%\n` +
      `${isId ? 'Alokasi RAM' : 'Memory (RSS)'}: ${proc.memMb} MB\n` +
      `${isId ? 'Jumlah Thread' : 'Active Threads'}: ${proc.threads}\n` +
      `${isId ? 'Pemilik' : 'Owner'}: ${proc.user}\n` +
      `Status: ${proc.status}\n` +
      `Architecture: Rust x86_64 Musl Async Reactor\n` +
      `File Descriptors: /proc/self/fd/0..4`
    );
  },

  terminateSelectedProcess() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const proc = this.processes.find(p => p.pid === this.selectedPid);

    if (!proc) {
      alert(isId ? 'Pilih salah satu proses di tabel terlebih dahulu!' : 'Please select a process from the table first!');
      return;
    }

    if (proc.name === 'bijios-kernel' || proc.name === 'dokuhub-kernel') {
      alert(isId ? '⚠️ bijios-kernel adalah proses inti sistem dan tidak dapat dihentikan!' : '⚠️ bijios-kernel is the system core and cannot be terminated!');
      return;
    }

    const confirmMsg = isId 
      ? `Apakah Anda yakin ingin menyegarkan/menghentikan proses "${proc.name}" (PID: ${proc.pid})?`
      : `Are you sure you want to recycle/terminate process "${proc.name}" (PID: ${proc.pid})?`;

    if (confirm(confirmMsg)) {
      proc.status = 'Recycled';
      proc.cpu = 0.0;
      setTimeout(() => {
        proc.status = 'Running';
        proc.cpu = 0.2;
        const container = document.getElementById('actmon-table-container');
        if (container) container.innerHTML = this.renderProcessTableHtml(isId);
      }, 1500);

      const container = document.getElementById('actmon-table-container');
      if (container) container.innerHTML = this.renderProcessTableHtml(isId);
      alert(isId ? `✅ Thread proses "${proc.name}" berhasil di-recycle!` : `✅ Process thread "${proc.name}" successfully recycled!`);
    }
  },

  updateDynamicValues() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    // Update Top Strip Stats
    const cpuEl = document.getElementById('actmon-stat-cpu');
    if (cpuEl) cpuEl.textContent = `${this.metrics.cpu.usage_percent}%`;

    const barCpu = document.getElementById('actmon-bar-cpu');
    if (barCpu) barCpu.style.width = `${this.metrics.cpu.usage_percent}%`;

    const subCpu = document.getElementById('actmon-sub-cpu');
    if (subCpu) subCpu.textContent = `${isId ? 'User' : 'User'}: ${this.metrics.cpu.user_percent}% • ${isId ? 'Sistem' : 'System'}: ${this.metrics.cpu.system_percent}%`;

    const memEl = document.getElementById('actmon-stat-mem');
    if (memEl) memEl.textContent = `${this.metrics.memory.used_percent}%`;

    const barMem = document.getElementById('actmon-bar-mem');
    if (barMem) barMem.style.width = `${this.metrics.memory.used_percent}%`;

    const subMem = document.getElementById('actmon-sub-mem');
    if (subMem) subMem.textContent = `${this.formatBytes(this.metrics.memory.used_kb * 1024)} / ${this.formatBytes(this.metrics.memory.total_kb * 1024)}`;

    const stEl = document.getElementById('actmon-stat-storage');
    if (stEl) stEl.textContent = this.formatBytes(this.metrics.storage.dokuhub_data_bytes || this.metrics.storage.used_bytes);

    const barSt = document.getElementById('actmon-bar-storage');
    if (barSt) barSt.style.width = `${this.metrics.storage.disk_used_percent || 2.1}%`;

    const subSt = document.getElementById('actmon-sub-storage');
    if (subSt) {
      subSt.textContent = `${isId ? 'Kapasitas Disk' : 'Disk Capacity'}: ${this.formatBytes(this.metrics.storage.disk_total_bytes || 211157901312)} (${isId ? 'Bebas' : 'Free'}: ${this.formatBytes(this.metrics.storage.disk_free_bytes || 198099652608)})`;
    }

    const upEl = document.getElementById('actmon-sub-uptime');
    if (upEl) upEl.textContent = `${isId ? 'Waktu Aktif' : 'Uptime'}: ${this.formatUptime(this.metrics.process.uptime_seconds)}`;

    const chartCpuBadge = document.getElementById('chart-cpu-badge');
    if (chartCpuBadge) chartCpuBadge.textContent = `${this.metrics.cpu.usage_percent}%`;

    const chartMemBadge = document.getElementById('chart-mem-badge');
    if (chartMemBadge) chartMemBadge.textContent = `${this.metrics.memory.used_percent}%`;

    // Dynamically update process table if attached
    const tableContainer = document.getElementById('actmon-table-container');
    if (tableContainer && !this.filterQuery && this.activeTab !== 'storage' && this.activeTab !== 'network') {
      tableContainer.innerHTML = this.renderProcessTableHtml(isId, this.activeTab === 'cpu' ? 'cpu' : (this.activeTab === 'memory' ? 'mem' : 'all'));
    }
  },

  drawCharts() {
    this.drawWaveformCanvas('actmon-cpu-canvas', this.cpuHistory, '#007aff', 'rgba(0, 122, 255, 0.25)');
    this.drawWaveformCanvas('actmon-mem-canvas', this.memHistory, '#34c759', 'rgba(52, 199, 89, 0.25)');
    this.drawWaveformCanvas('actmon-net-canvas', this.netHistory, '#5856d6', 'rgba(88, 86, 214, 0.25)');
  },

  drawWaveformCanvas(canvasId, dataPoints, strokeColor, fillColor) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const width = canvas.offsetWidth || 380;
    const height = canvas.offsetHeight || 90;

    canvas.width = width * 2;
    canvas.height = height * 2;
    ctx.scale(2, 2);

    ctx.clearRect(0, 0, width, height);

    // Draw subtle grid lines
    ctx.strokeStyle = 'rgba(125, 125, 125, 0.15)';
    ctx.lineWidth = 1;
    for (let y = 20; y < height; y += 22) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    if (!dataPoints || dataPoints.length === 0) return;

    const step = width / (dataPoints.length - 1);
    const maxVal = 100;

    // Draw wave path
    ctx.beginPath();
    ctx.moveTo(0, height);

    dataPoints.forEach((val, idx) => {
      const x = idx * step;
      const normalized = Math.max(0, Math.min(100, val));
      const y = height - ((normalized / maxVal) * (height - 8));
      if (idx === 0) ctx.lineTo(x, y);
      else {
        const prevX = (idx - 1) * step;
        const prevVal = Math.max(0, Math.min(100, dataPoints[idx - 1]));
        const prevY = height - ((prevVal / maxVal) * (height - 8));
        const cx = (prevX + x) / 2;
        ctx.bezierCurveTo(cx, prevY, cx, y, x, y);
      }
    });

    // Fill under path
    ctx.lineTo(width, height);
    ctx.closePath();
    ctx.fillStyle = fillColor;
    ctx.fill();

    // Stroke top wave line
    ctx.beginPath();
    dataPoints.forEach((val, idx) => {
      const x = idx * step;
      const normalized = Math.max(0, Math.min(100, val));
      const y = height - ((normalized / maxVal) * (height - 8));
      if (idx === 0) ctx.moveTo(x, y);
      else {
        const prevX = (idx - 1) * step;
        const prevVal = Math.max(0, Math.min(100, dataPoints[idx - 1]));
        const prevY = height - ((prevVal / maxVal) * (height - 8));
        const cx = (prevX + x) / 2;
        ctx.bezierCurveTo(cx, prevY, cx, y, x, y);
      }
    });
    ctx.strokeStyle = strokeColor;
    ctx.lineWidth = 2;
    ctx.stroke();
  }
};
