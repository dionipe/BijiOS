/**
 * DokuHub Settings - Users Management (RBAC CRUD), Profiles, API Docs & TOON v4 Inspector
 * Complete English UI with real Add/Edit/Delete user operations and individual user profile switching.
 */

window.DokuSettings = {
  container: null,
  activeTab: 'profile',
  users: [],
  tenants: [],
  apiKeys: [],
  toonStats: [],
  shareApprovals: [],
  shareApprovalFilter: 'all',
  editingUserId: null,

  isImageAvatar(avatar) {
    if (!avatar || typeof avatar !== 'string') return false;
    const clean = avatar.split('?')[0].toLowerCase();
    return avatar.startsWith('data:image') ||
           avatar.startsWith('/api/') ||
           avatar.includes('/avatar') ||
           avatar.startsWith('http://') ||
           avatar.startsWith('https://') ||
           clean.endsWith('.png') ||
           clean.endsWith('.jpg') ||
           clean.endsWith('.jpeg') ||
           clean.endsWith('.webp') ||
           clean.endsWith('.gif') ||
           clean.endsWith('.svg');
  },

  renderAvatarHtml(avatar, color, initials, size = 32) {
    const isImg = this.isImageAvatar(avatar);
    const bg = isImg ? 'transparent' : (color || '#007aff');
    const safeInitials = initials || 'US';
    const fallbackText = (avatar && avatar.length <= 4) ? avatar : safeInitials;
    return `
      <div style="width:${size}px; height:${size}px; border-radius:50%; background:${bg}; color:#fff; display:flex; align-items:center; justify-content:center; font-size:${Math.round(size * 0.38)}px; font-weight:700; overflow:hidden; flex-shrink:0; border:1.5px solid rgba(255,255,255,0.25);">
        ${isImg 
          ? `<img src="${avatar}" alt="" style="width:100%; height:100%; border-radius:50%; object-fit:cover; display:block;" onerror="this.onerror=null; this.parentElement.style.background='${color || '#007aff'}'; this.parentElement.textContent='${safeInitials}';">` 
          : fallbackText}
      </div>
    `;
  },

  handleQuotaSelectChange(type) {
    const select = document.getElementById(`${type}-user-quota`);
    const customWrap = document.getElementById(`${type}-user-quota-custom-wrap`);
    const customInput = document.getElementById(`${type}-user-quota-custom`);
    if (select && customWrap) {
      if (select.value === 'custom') {
        customWrap.style.display = 'block';
        if (customInput) customInput.focus();
      } else {
        customWrap.style.display = 'none';
      }
    }
  },

  getQuotaBytesValue(type) {
    const select = document.getElementById(`${type}-user-quota`);
    const customInput = document.getElementById(`${type}-user-quota-custom`);
    if (!select) return '10737418240';
    if (select.value === 'custom') {
      const gb = Math.max(1, parseInt(customInput && customInput.value ? customInput.value : '10') || 10);
      return String(gb * 1024 * 1024 * 1024);
    }
    return select.value;
  },

  async init(containerEl) {
    this.container = containerEl;
    await this.loadAllData();
    this.render();
  },

  async loadAllData() {
    try {
      const headers = {
        'X-Tenant-Id': (DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : (DokuHub.currentUser ? DokuHub.currentUser.tenant_id : ''),
        'X-User-Id': DokuHub.currentUser ? DokuHub.currentUser.id : ''
      };
      const [uRes, tRes, kRes, toonRes] = await Promise.all([
        fetch(`/api/users?tenant_id=${headers['X-Tenant-Id']}`, { headers }),
        fetch('/api/tenants', { headers }),
        fetch('/api/settings/api-keys', { headers }),
        fetch('/api/settings/toon-inspect', { headers })
      ]);
      this.users = await uRes.json();
      this.tenants = await tRes.json();
      this.apiKeys = await kRes.json();
      this.toonStats = await toonRes.json();
    } catch (e) {
      console.error("Failed to load settings data:", e);
      this.users = [];
      this.tenants = [];
      this.apiKeys = [];
      this.toonStats = [];
    }
  },

  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const cur = DokuHub.currentUser || { role: 'Member' };
    const isSuperAdmin = cur.role === 'SuperAdmin';
    const isAdmin = isSuperAdmin || cur.role === 'Admin';

    this.container.innerHTML = `
      <div style="display:flex; height:100%; background:var(--window-bg);">
        <!-- Sidebar Navigation -->
        <div style="width:230px; background:var(--sidebar-bg); border-right:1px solid var(--window-border); padding:16px 10px; display:flex; flex-direction:column; gap:4px; flex-shrink:0;">
          <div class="sidebar-section-title">${isId ? 'Preferensi Pribadi' : 'Personal Preferences'}</div>
          <div class="sidebar-item ${this.activeTab === 'profile' ? 'active' : ''}" onclick="DokuSettings.showTab('profile')">
            👤 ${isId ? 'Profil & Kata Sandi' : 'Profile & Password'}
          </div>
          <div class="sidebar-item ${this.activeTab === 'appearance' ? 'active' : ''}" onclick="DokuSettings.showTab('appearance')">
            🎨 ${isId ? 'Tampilan & Docking' : 'Appearance & Dock'}
          </div>
          <div class="sidebar-item ${this.activeTab === 'approvals' ? 'active' : ''}" onclick="DokuSettings.showTab('approvals')">
            ✅ ${isId ? 'Persetujuan Share' : 'Share Approvals'}
          </div>

          ${isAdmin ? `
            <div style="height:1px; background:var(--window-border); margin:8px 0;"></div>
            <div class="sidebar-section-title">${isId ? 'Sistem & Organisasi' : 'System & Workspace'}</div>
            
            ${isSuperAdmin ? `
              <div class="sidebar-item ${this.activeTab === 'tenants' ? 'active' : ''}" onclick="DokuSettings.showTab('tenants')">
                🏢 ${isId ? 'Kelola Organisasi (Tenant)' : 'Tenant Management'}
              </div>
            ` : ''}

            <div class="sidebar-item ${this.activeTab === 'rbac' ? 'active' : ''}" onclick="DokuSettings.showTab('rbac')">
              👥 ${isId ? 'Manajemen User & RBAC' : 'Users & RBAC'}
            </div>
            <div class="sidebar-item ${this.activeTab === 'audit' ? 'active' : ''}" onclick="DokuSettings.showTab('audit')">
              🛡️ ${isId ? 'Audit Trail & Keamanan' : 'Security & Audit Trail'}
            </div>
            <div class="sidebar-item ${this.activeTab === 'apidocs' ? 'active' : ''}" onclick="DokuSettings.showTab('apidocs')">
              📖 ${isId ? 'Dokumentasi API' : 'API Documentation'}
            </div>
            <div class="sidebar-item ${this.activeTab === 'apikeys' ? 'active' : ''}" onclick="DokuSettings.showTab('apikeys')">
              🔑 ${isId ? 'Kunci Integrasi API' : 'API Integration Keys'}
            </div>
            <div class="sidebar-item ${this.activeTab === 'toon' ? 'active' : ''}" onclick="DokuSettings.showTab('toon')">
              ⚡ TOON v4 Database
            </div>
          ` : ''}
        </div>

        <!-- Main Settings Pane -->
        <div style="flex:1; overflow-y:auto; padding:24px;" id="settings-pane-content">
          ${this.renderTabContent()}
        </div>
      </div>

      <!-- Add User Modal -->
      <div id="modal-add-user" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(10px); z-index:999999; align-items:center; justify-content:center;">
        <div style="width:440px; background:var(--window-bg); backdrop-filter:blur(30px); border:1px solid var(--window-border); border-radius:16px; padding:24px; box-shadow:0 30px 70px rgba(0,0,0,0.4);">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
            <h3 style="font-size:16px; font-weight:700;">${isId ? 'Tambah Pengguna Baru' : 'Add New Workspace User'}</h3>
            <button class="traffic-btn btn-close" onclick="DokuSettings.closeAddUserModal()"></button>
          </div>
          <div style="display:flex; flex-direction:column; gap:12px; margin-bottom:20px;">
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Nama Lengkap:' : 'Full Name:'}</label>
              <input type="text" id="add-user-name" placeholder="${isId ? 'mis. Sarah Connor' : 'e.g. Sarah Connor'}" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Alamat Email:' : 'Email Address:'}</label>
              <input type="email" id="add-user-email" placeholder="sarah@acme.com" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Organisasi / Tenant:' : 'Organization / Tenant:'}</label>
              <select id="add-user-tenant" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                ${this.tenants.map(t => `<option value="${t.id}" ${t.id === DokuHub.activeTenant.id ? 'selected' : ''}>🏢 ${t.name}</option>`).join('')}
              </select>
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Peran (Role):' : 'Role:'}</label>
              <select id="add-user-role" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                <option value="Staff">Staff (${isId ? 'Mengajukan Share ke Supervisor' : 'Submits Share Requests to Supervisor'})</option>
                <option value="Supervisor">Supervisor (${isId ? 'Approval Share Bagiannya' : 'Department Share Approver'})</option>
                <option value="Admin">Admin Tenant (${isId ? 'Kelola Tenant & Semua Bagian' : 'Tenant Admin & All Departments'})</option>
                <option value="SuperAdmin">SuperAdmin (${isId ? 'Otoritas Seluruh Sistem' : 'System-wide Authority'})</option>
                <option value="Guest">Guest (${isId ? 'Hanya Lihat / Eksternal' : 'Read-only / External'})</option>
              </select>
            </div>
            <div style="display:grid; grid-template-columns:1fr 1fr; gap:8px;">
              <div>
                <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Bagian / Divisi:' : 'Department:'}</label>
                <input type="text" id="add-user-dept" placeholder="${isId ? 'mis. Bagian A / Bagian B' : 'e.g. Dept A / Dept B'}" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:12.5px; background:var(--card-bg); color:var(--text-main);">
              </div>
              <div>
                <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Atasan (Supervisor):' : 'Supervisor:'}</label>
                <select id="add-user-supervisor" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:12.5px; background:var(--card-bg); color:var(--text-main);">
                  <option value="">— ${isId ? 'Tanpa Atasan' : 'None'} —</option>
                  ${this.users.filter(u => ['Supervisor', 'Admin', 'SuperAdmin'].includes(u.role)).map(sup => `<option value="${sup.id}">${sup.name} (${sup.role}${sup.department ? ' - ' + sup.department : ''})</option>`).join('')}
                </select>
              </div>
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Batas Kuota Penyimpanan:' : 'Storage Quota Limit:'}</label>
              <div style="display:flex; gap:8px;">
                <select id="add-user-quota" style="flex:1; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);" onchange="DokuSettings.handleQuotaSelectChange('add')">
                  <option value="1073741824">1 GB</option>
                  <option value="5368709120">5 GB</option>
                  <option value="10737418240" selected>10 GB</option>
                  <option value="21474836480">20 GB</option>
                  <option value="32212254720">30 GB</option>
                  <option value="custom">${isId ? 'Kustom (GB)...' : 'Custom (GB)...'}</option>
                </select>
                <div id="add-user-quota-custom-wrap" style="display:none; width:110px; position:relative;">
                  <input type="number" id="add-user-quota-custom" min="1" max="10000" placeholder="GB" value="10" style="width:100%; padding:8px 24px 8px 10px; border:1px solid var(--accent-color); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                  <span style="position:absolute; right:8px; top:8px; font-size:11px; font-weight:700; color:var(--text-muted);">GB</span>
                </div>
              </div>
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Kata Sandi Awal:' : 'Initial Password:'}</label>
              <input type="password" id="add-user-pass" value="password123" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
          </div>
          <div style="display:flex; justify-content:flex-end; gap:8px;">
            <button class="mac-btn" onclick="DokuSettings.closeAddUserModal()">${isId ? 'Batal' : 'Cancel'}</button>
            <button class="mac-btn primary" onclick="DokuSettings.submitAddUser()">${isId ? 'Buat Pengguna' : 'Create User'}</button>
          </div>
        </div>
      </div>

      <!-- Edit User Modal -->
      <div id="modal-edit-user" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(10px); z-index:999999; align-items:center; justify-content:center;">
        <div style="width:440px; background:var(--window-bg); backdrop-filter:blur(30px); border:1px solid var(--window-border); border-radius:16px; padding:24px; box-shadow:0 30px 70px rgba(0,0,0,0.4);">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:12px;">
            <h3 style="font-size:16px; font-weight:700;">${isId ? 'Edit Profil Pengguna' : 'Edit User Profile'}</h3>
            <button class="traffic-btn btn-close" onclick="DokuSettings.closeEditUserModal()"></button>
          </div>
          <div id="edit-user-avatar-badge" style="display:flex; align-items:center; gap:12px; padding:10px 14px; background:rgba(0,122,255,0.08); border:1px solid rgba(0,122,255,0.2); border-radius:10px; margin-bottom:14px;"></div>
          <div style="display:flex; flex-direction:column; gap:12px; margin-bottom:20px;">
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Nama Lengkap:' : 'Full Name:'}</label>
              <input type="text" id="edit-user-name" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Alamat Email:' : 'Email Address:'}</label>
              <input type="email" id="edit-user-email" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Peran (Role):' : 'Role:'}</label>
              <select id="edit-user-role" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                <option value="SuperAdmin">SuperAdmin</option>
                <option value="Admin">Admin Tenant</option>
                <option value="Supervisor">Supervisor</option>
                <option value="Staff">Staff</option>
                <option value="Guest">Guest</option>
              </select>
            </div>
            <div style="display:grid; grid-template-columns:1fr 1fr; gap:8px;">
              <div>
                <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Bagian / Divisi:' : 'Department:'}</label>
                <input type="text" id="edit-user-dept" placeholder="${isId ? 'mis. Bagian A' : 'e.g. Dept A'}" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:12.5px; background:var(--card-bg); color:var(--text-main);">
              </div>
              <div>
                <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Atasan (Supervisor):' : 'Supervisor:'}</label>
                <select id="edit-user-supervisor" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:12.5px; background:var(--card-bg); color:var(--text-main);">
                  <option value="">— ${isId ? 'Tanpa Atasan' : 'None'} —</option>
                  ${this.users.filter(u => ['Supervisor', 'Admin', 'SuperAdmin'].includes(u.role)).map(sup => `<option value="${sup.id}">${sup.name} (${sup.role}${sup.department ? ' - ' + sup.department : ''})</option>`).join('')}
                </select>
              </div>
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Organisasi / Tenant:' : 'Organization / Tenant:'}</label>
              <select id="edit-user-tenant" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                ${this.tenants.map(t => `<option value="${t.id}">🏢 ${t.name} (${t.slug})</option>`).join('')}
              </select>
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Status Akun:' : 'Status:'}</label>
              <select id="edit-user-status" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                <option value="active">${isId ? 'Aktif' : 'Active'}</option>
                <option value="suspended">${isId ? 'Ditangguhkan (Suspended)' : 'Suspended'}</option>
              </select>
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Batas Kuota Penyimpanan:' : 'Storage Quota Limit:'}</label>
              <div style="display:flex; gap:8px;">
                <select id="edit-user-quota" style="flex:1; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);" onchange="DokuSettings.handleQuotaSelectChange('edit')">
                  <option value="1073741824">1 GB</option>
                  <option value="5368709120">5 GB</option>
                  <option value="10737418240">10 GB</option>
                  <option value="21474836480">20 GB</option>
                  <option value="32212254720">30 GB</option>
                  <option value="custom">${isId ? 'Kustom (GB)...' : 'Custom (GB)...'}</option>
                </select>
                <div id="edit-user-quota-custom-wrap" style="display:none; width:110px; position:relative;">
                  <input type="number" id="edit-user-quota-custom" min="1" max="10000" placeholder="GB" value="10" style="width:100%; padding:8px 24px 8px 10px; border:1px solid var(--accent-color); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                  <span style="position:absolute; right:8px; top:8px; font-size:11px; font-weight:700; color:var(--text-muted);">GB</span>
                </div>
              </div>
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Kata Sandi Baru (kosongkan jika tidak diubah):' : 'New Password (leave empty to keep current):'}</label>
              <input type="password" id="edit-user-pass" placeholder="••••••••" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
          </div>
          <div style="display:flex; justify-content:flex-end; gap:8px;">
            <button class="mac-btn" onclick="DokuSettings.closeEditUserModal()">${isId ? 'Batal' : 'Cancel'}</button>
            <button class="mac-btn primary" onclick="DokuSettings.submitEditUser()">${isId ? 'Simpan Perubahan' : 'Save Changes'}</button>
          </div>
        </div>
      </div>

      <!-- Add Tenant Modal (SuperAdmin Only) -->
      <div id="modal-add-tenant" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(10px); z-index:999999; align-items:center; justify-content:center;">
        <div style="width:440px; background:var(--window-bg); backdrop-filter:blur(30px); border:1px solid var(--window-border); border-radius:16px; padding:24px; box-shadow:0 30px 70px rgba(0,0,0,0.4);">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
            <h3 style="font-size:16px; font-weight:700;">${isId ? 'Tambah Organisasi / Tenant Baru' : 'Add New Organization / Tenant'}</h3>
            <button class="traffic-btn btn-close" onclick="DokuSettings.closeAddTenantModal()"></button>
          </div>
          <div style="display:flex; flex-direction:column; gap:12px; margin-bottom:20px;">
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Nama Organisasi / Perusahaan:' : 'Organization Name:'}</label>
              <input type="text" id="add-tenant-name" placeholder="${isId ? 'mis. PT Digital Nusantara' : 'e.g. Acme Corp'}" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Slug Identitas (URL identifier):' : 'Slug Identifier:'}</label>
              <input type="text" id="add-tenant-slug" placeholder="${isId ? 'mis. nusantara (kosongkan untuk auto)' : 'e.g. acme (leave blank for auto)'}" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Paket / Lisensi Tenant:' : 'Tenant Plan / Tier:'}</label>
              <select id="add-tenant-plan" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                <option value="Enterprise Plus">Enterprise Plus (${isId ? 'Tanpa Batas' : 'Unlimited'})</option>
                <option value="Pro Team">Pro Team (${isId ? 'Kolaboratif' : 'Collaborative'})</option>
                <option value="Standard">Standard (${isId ? 'Standar' : 'Basic'})</option>
              </select>
            </div>
          </div>
          <div style="display:flex; justify-content:flex-end; gap:8px;">
            <button class="mac-btn" onclick="DokuSettings.closeAddTenantModal()">${isId ? 'Batal' : 'Cancel'}</button>
            <button class="mac-btn primary" onclick="DokuSettings.submitAddTenant()">${isId ? 'Buat Tenant' : 'Create Tenant'}</button>
          </div>
        </div>
      </div>

      <!-- Edit Tenant Modal (SuperAdmin Only) -->
      <div id="modal-edit-tenant" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(10px); z-index:999999; align-items:center; justify-content:center;">
        <div style="width:440px; background:var(--window-bg); backdrop-filter:blur(30px); border:1px solid var(--window-border); border-radius:16px; padding:24px; box-shadow:0 30px 70px rgba(0,0,0,0.4);">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
            <h3 style="font-size:16px; font-weight:700;">${isId ? 'Edit Detail Organisasi / Tenant' : 'Edit Organization / Tenant'}</h3>
            <button class="traffic-btn btn-close" onclick="DokuSettings.closeEditTenantModal()"></button>
          </div>
          <div style="display:flex; flex-direction:column; gap:12px; margin-bottom:20px;">
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Nama Organisasi:' : 'Organization Name:'}</label>
              <input type="text" id="edit-tenant-name" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Slug Identitas:' : 'Slug Identifier:'}</label>
              <input type="text" id="edit-tenant-slug" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Paket / Lisensi Tenant:' : 'Tenant Plan / Tier:'}</label>
              <select id="edit-tenant-plan" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
                <option value="Enterprise Plus">Enterprise Plus (${isId ? 'Tanpa Batas' : 'Unlimited'})</option>
                <option value="Pro Team">Pro Team (${isId ? 'Kolaboratif' : 'Collaborative'})</option>
                <option value="Standard">Standard (${isId ? 'Standar' : 'Basic'})</option>
              </select>
            </div>
          </div>
          <div style="display:flex; justify-content:flex-end; gap:8px;">
            <button class="mac-btn" onclick="DokuSettings.closeEditTenantModal()">${isId ? 'Batal' : 'Cancel'}</button>
            <button class="mac-btn primary" onclick="DokuSettings.submitEditTenant()">${isId ? 'Simpan Perubahan' : 'Save Changes'}</button>
          </div>
        </div>
      </div>
    `;
  },

  showTab(tab) {
    this.activeTab = tab;
    if (tab === 'audit') {
      this.loadAuditLogs();
    } else if (tab === 'approvals') {
      this.loadShareApprovals();
    }
    this.render();
  },

  renderTabContent() {
    const cur = DokuHub.currentUser || { role: 'Member' };
    const isSuperAdmin = cur.role === 'SuperAdmin';
    const isAdmin = isSuperAdmin || cur.role === 'Admin';

    if (this.activeTab === 'profile') return this.renderProfileTab();
    if (this.activeTab === 'appearance') return this.renderAppearanceTab();
    if (this.activeTab === 'approvals') return this.renderShareApprovalsTab();
    if (this.activeTab === 'tenants' && isSuperAdmin) return this.renderTenantsTab();
    if (this.activeTab === 'rbac' && isAdmin) return this.renderRbacTab();
    if (this.activeTab === 'audit' && isAdmin) return this.renderAuditTab();
    if (this.activeTab === 'apidocs' && isAdmin) return this.renderApiDocsTab();
    if (this.activeTab === 'apikeys' && isAdmin) return this.renderApiKeysTab();
    if (this.activeTab === 'toon' && isAdmin) return this.renderToonTab();

    // Default fallback for Member / Guest
    this.activeTab = 'profile';
    return this.renderProfileTab();
  },

  /* ------------------------------------------------------------------------
     1. User Profile & Preferences Tab
     ------------------------------------------------------------------------ */
  renderProfileTab() {
    const cur = DokuHub.currentUser || { id: 'usr_dionipe_849', name: 'dionipe', email: 'dionipe@gmail.com', role: 'SuperAdmin', avatar: 'DI', color: '#007aff' };
    const curUserInDb = this.users.find(u => u.id === cur.id) || cur;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    const curInitials = (cur.name || 'User').split(' ').map(n=>n[0]).join('').slice(0, 2).toUpperCase();
    const effectiveAvatar = cur.avatar || curUserInDb.avatar || curInitials;
    const effectiveColor = cur.color || curUserInDb.color || '#007aff';
    const isImgAvatar = this.isImageAvatar(effectiveAvatar);

    return `
      <div style="max-width:860px;">
        <!-- Header Banner -->
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:20px;">
          <div>
            <h2 style="font-size:20px; font-weight:700;">${isId ? 'Preferensi Profil Pengguna' : 'User Profile & Preferences'}</h2>
            <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
              ${isId ? 'Kelola identitas, foto avatar, kata sandi, dan organisasi/tenant yang terhubung.' : 'Manage your identity, avatar photo, password, and connected organizations/tenants.'}
            </p>
          </div>
          <span style="font-size:11px; padding:4px 10px; border-radius:12px; background:rgba(0,122,255,0.12); color:var(--accent-color); font-weight:700;">
            ${cur.role}
          </span>
        </div>

        <!-- 1. Identity & Avatar Customization Card -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:14px; padding:20px; margin-bottom:20px;">
          <h3 style="font-size:15px; font-weight:700; margin-bottom:16px; display:flex; align-items:center; gap:8px;">
            <span>👤</span> ${isId ? 'Identitas & Kustomisasi Avatar' : 'Identity & Avatar Customization'}
          </h3>

          <div style="display:flex; gap:24px; align-items:flex-start; flex-wrap:wrap;">
            <!-- Big Avatar Preview -->
            <div style="display:flex; flex-direction:column; align-items:center; gap:10px;">
              <div id="profile-avatar-preview" style="width:84px; height:84px; border-radius:50%; background:${isImgAvatar ? 'transparent' : effectiveColor}; color:#fff; display:flex; align-items:center; justify-content:center; font-size:32px; font-weight:700; box-shadow:0 8px 24px rgba(0,0,0,0.25); border:3px solid var(--window-bg); overflow:hidden;">
                ${isImgAvatar ? `<img src="${effectiveAvatar}" alt="" style="width:100%; height:100%; border-radius:50%; object-fit:cover; display:block;" onerror="this.onerror=null; this.parentElement.style.background='${effectiveColor}'; this.parentElement.textContent='${curInitials}';">` : ((effectiveAvatar && effectiveAvatar.length <= 4) ? effectiveAvatar : curInitials)}
              </div>
              <div style="display:flex; gap:6px;">
                <label class="mac-btn" style="cursor:pointer; font-size:11px; padding:3px 8px;">
                  📷 ${isId ? 'Unggah Foto' : 'Upload'}
                  <input type="file" id="profile-avatar-file" accept="image/*" style="display:none;" onchange="DokuSettings.handleAvatarUpload(event)">
                </label>
                <button class="mac-btn" style="font-size:11px; padding:3px 8px;" onclick="DokuSettings.resetAvatarDefault()">
                  ↺ Reset
                </button>
              </div>
            </div>

            <!-- Avatar Pickers & Color Swatches -->
            <div style="flex:1; min-width:280px;">
              <!-- Quick Emoji / Symbol Presets -->
              <div style="margin-bottom:14px;">
                <label style="font-size:11px; font-weight:600; display:block; margin-bottom:6px; color:var(--text-muted);">
                  ${isId ? 'Pilih Ikon / Emoji Avatar Cepat:' : 'Choose Quick Avatar Icon / Emoji:'}
                </label>
                <div style="display:flex; gap:8px; flex-wrap:wrap;">
                  ${['👤', '👩‍💻', '👨‍💻', '🚀', '⚡', '🛡️', '💼', '🎯', '🎨', '🐱', '🦊', '🦁'].map(em => `
                    <button type="button" class="mac-btn" style="width:34px; height:34px; padding:0; font-size:16px; border-radius:8px;" onclick="DokuSettings.selectAvatarPreset('${em}')">${em}</button>
                  `).join('')}
                </div>
              </div>

              <!-- Avatar Background Color Palette -->
              <div>
                <label style="font-size:11px; font-weight:600; display:block; margin-bottom:6px; color:var(--text-muted);">
                  ${isId ? 'Warna Latar Avatar:' : 'Avatar Background Color:'}
                </label>
                <div style="display:flex; gap:8px; align-items:center;">
                  ${['#007aff', '#10b981', '#8b5cf6', '#f59e0b', '#ef4444', '#ec4899', '#6366f1', '#475569'].map(c => `
                    <div onclick="DokuSettings.setAvatarColor('${c}')" style="width:24px; height:24px; border-radius:50%; background:${c}; cursor:pointer; border:2px solid ${(effectiveColor === c) ? '#fff' : 'transparent'}; box-shadow:0 2px 6px rgba(0,0,0,0.2);"></div>
                  `).join('')}
                </div>
              </div>
            </div>
          </div>

          <div style="height:1px; background:var(--window-border); margin:20px 0;"></div>

          <!-- User Form Fields -->
          <div style="display:grid; grid-template-columns:1fr 1fr; gap:16px;">
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Nama Lengkap' : 'Full Name'}:</label>
              <input type="text" id="profile-name-input" value="${curUserInDb.name || cur.name}" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Alamat Email' : 'Email Address'}:</label>
              <input type="email" id="profile-email-input" value="${curUserInDb.email || cur.email}" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
          </div>

          <div style="margin-top:16px; display:flex; justify-content:flex-end;">
            <button class="mac-btn primary" onclick="DokuSettings.saveProfileBasicInfo()">💾 ${isId ? 'Simpan Data Profil' : 'Save Profile Changes'}</button>
          </div>
        </div>

        <!-- 2. Ganti Password Card -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:14px; padding:20px; margin-bottom:20px;">
          <h3 style="font-size:15px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:8px;">
            <span>🔑</span> ${isId ? 'Ganti Kata Sandi (Password)' : 'Change Account Password'}
          </h3>
          <p style="font-size:12px; color:var(--text-muted); margin-bottom:16px;">
            ${isId ? 'Amankan akun Anda dengan mengganti kata sandi secara berkala (minimal 4 karakter).' : 'Keep your sovereign account secure by updating your password.'}
          </p>

          <div style="display:grid; grid-template-columns:1fr 1fr 1fr; gap:14px;">
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Kata Sandi Saat Ini' : 'Current Password'}:</label>
              <input type="password" id="profile-old-pass" placeholder="••••••••" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Kata Sandi Baru' : 'New Password'}:</label>
              <input type="password" id="profile-new-pass" placeholder="••••••••" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
            <div>
              <label style="font-size:11px; font-weight:600; display:block; margin-bottom:4px;">${isId ? 'Konfirmasi Kata Sandi' : 'Confirm New Password'}:</label>
              <input type="password" id="profile-confirm-pass" placeholder="••••••••" style="width:100%; padding:8px 10px; border:1px solid var(--window-border); border-radius:6px; font-size:13px; background:var(--card-bg); color:var(--text-main);">
            </div>
          </div>

          <div style="margin-top:16px; display:flex; justify-content:space-between; align-items:center;">
            <div id="password-change-feedback" style="font-size:12px;"></div>
            <button class="mac-btn primary" onclick="DokuSettings.submitChangePassword()">🔒 ${isId ? 'Perbarui Kata Sandi' : 'Update Password'}</button>
          </div>
        </div>

        <!-- 3. Connected Tenants / Organizations Card -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:14px; padding:20px; margin-bottom:20px;">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:14px;">
            <div>
              <h3 style="font-size:15px; font-weight:700; display:flex; align-items:center; gap:8px;">
                <span>🏢</span> ${isId ? 'Organisasi & Tenant yang Terhubung' : 'Connected Tenants & Organizations'}
              </h3>
              <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
                ${isId ? 'Data dan berkas terisolasi secara mandiri pada setiap tenant yang Anda akses.' : 'Workspaces and storage are strictly isolated per tenant.'}
              </p>
            </div>
            <span style="font-size:12px; font-weight:600; color:var(--accent-color); background:rgba(0,122,255,0.1); padding:4px 10px; border-radius:8px;">
              Active: <strong>${DokuHub.activeTenant.name}</strong>
            </span>
          </div>

          <div style="display:flex; flex-direction:column; gap:10px;">
            ${this.tenants.map(t => {
              const isActive = (t.id === DokuHub.activeTenant.id);
              return `
                <div style="display:flex; justify-content:space-between; align-items:center; padding:12px 14px; border:1px solid ${isActive ? 'var(--accent-color)' : 'var(--window-border)'}; border-radius:10px; background:${isActive ? 'rgba(0,122,255,0.06)' : 'transparent'};">
                  <div style="display:flex; align-items:center; gap:12px;">
                    <div style="width:36px; height:36px; border-radius:8px; background:var(--window-border); display:flex; align-items:center; justify-content:center; font-size:18px;">
                      🏢
                    </div>
                    <div>
                      <div style="font-weight:700; font-size:13px;">${t.name} ${isActive ? `<span style="font-size:10px; color:var(--accent-color); font-weight:700; margin-left:6px;">● ACTIVE</span>` : ''}</div>
                      <div style="font-size:11px; color:var(--text-muted);">Tenant ID: <code>${t.id}</code> • Plan: <strong>${t.plan || 'Enterprise'}</strong></div>
                    </div>
                  </div>
                  <div>
                    ${!isActive ? `
                      <button class="mac-btn primary" style="font-size:11px; padding:4px 10px;" onclick="DokuSettings.switchTenant('${t.id}')">
                        🔁 ${isId ? 'Beralih ke Tenant Ini' : 'Switch Tenant'}
                      </button>
                    ` : `
                      <span style="font-size:11px; color:#10b981; font-weight:600;">✓ ${isId ? 'Sedang Digunakan' : 'Current Active'}</span>
                    `}
                  </div>
                </div>
              `;
            }).join('')}
          </div>
        </div>
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     2. Appearance, Themes & Dock Preferences Tab
     ------------------------------------------------------------------------ */
  renderAppearanceTab() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curTheme = document.documentElement.getAttribute('data-theme') || 'light';
    const curAccent = localStorage.getItem('dokuhub_accent_color') || '#007aff';
    const curDockSize = parseInt(localStorage.getItem('dokuhub_dock_size') || '52', 10);
    const curDockMagnify = localStorage.getItem('dokuhub_dock_magnify') !== 'false';
    const curDockPos = localStorage.getItem('dokuhub_dock_position') || 'bottom';

    const accentPalettes = [
      { name: 'macOS Blue', color: '#007aff' },
      { name: 'Purple', color: '#af52de' },
      { name: 'Pink', color: '#ff2d55' },
      { name: 'Red', color: '#ff3b30' },
      { name: 'Orange', color: '#ff9500' },
      { name: 'Yellow', color: '#ffcc00' },
      { name: 'Green', color: '#34c759' },
      { name: 'Graphite', color: '#8e8e93' }
    ];

    return `
      <div style="max-width:860px;">
        <!-- Header -->
        <div style="margin-bottom:20px;">
          <h2 style="font-size:20px; font-weight:700;">${isId ? 'Tampilan, Tema & Pengaturan Dock' : 'Appearance, Themes & Dock Preferences'}</h2>
          <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
            ${isId ? 'Sesuaikan gaya tema, warna aksen khas macOS, serta ukuran dan efek pembesaran Dock sesuai selera Anda.' : 'Customize appearance theme, accent colors, and dock size & magnification.'}
          </p>
        </div>

        <!-- 1. Docking Configuration Card (Perbesar & Perkecil Dock) -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:14px; padding:20px; margin-bottom:20px;">
          <h3 style="font-size:15px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:8px;">
            <span>⚓</span> ${isId ? 'Pengaturan Ukuran & Efek Docking' : 'Dock Size & Magnification'}
          </h3>
          <p style="font-size:12px; color:var(--text-muted); margin-bottom:16px;">
            ${isId ? 'Perkecil atau perbesar ukuran ikon aplikasi pada dock di bagian bawah desktop.' : 'Adjust dock icon sizing and magnification to fit your screen preferences.'}
          </p>

          <!-- Slider Ukuran Dock -->
          <div style="margin-bottom:20px;">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px;">
              <label style="font-size:12px; font-weight:700;">${isId ? 'Ukuran Dock (Dock Size):' : 'Dock Size:'} <span id="dock-size-label" style="color:var(--accent-color);">${curDockSize} px</span></label>
              <!-- Quick Presets -->
              <div style="display:flex; gap:6px;">
                <button class="mac-btn" style="font-size:11px; padding:2px 8px;" onclick="DokuSettings.applyDockSize(40)">${isId ? 'Kecil (40px)' : 'Small (40px)'}</button>
                <button class="mac-btn" style="font-size:11px; padding:2px 8px;" onclick="DokuSettings.applyDockSize(52)">${isId ? 'Sedang (52px)' : 'Medium (52px)'}</button>
                <button class="mac-btn" style="font-size:11px; padding:2px 8px;" onclick="DokuSettings.applyDockSize(68)">${isId ? 'Besar (68px)' : 'Large (68px)'}</button>
              </div>
            </div>
            <div style="display:flex; align-items:center; gap:12px;">
              <span style="font-size:11px; color:var(--text-muted);">${isId ? 'Kecil' : 'Small'}</span>
              <input type="range" id="dock-size-slider" min="36" max="76" step="2" value="${curDockSize}" style="flex:1; cursor:pointer;" oninput="DokuSettings.applyDockSize(this.value)">
              <span style="font-size:11px; color:var(--text-muted);">${isId ? 'Besar' : 'Large'}</span>
            </div>
          </div>

          <!-- Efek Pembesaran (Magnification on Hover) -->
          <div style="display:flex; align-items:center; justify-content:space-between; padding-top:14px; border-top:1px solid var(--window-border); margin-bottom:16px;">
            <div>
              <div style="font-size:13px; font-weight:600;">${isId ? 'Efek Pembesaran (Hover Magnification)' : 'Magnification on Hover'}</div>
              <div style="font-size:11px; color:var(--text-muted);">${isId ? 'Ikon membesar saat kursor mouse diarahkan ke aplikasi di dock.' : 'Icons zoom smoothly when hovered.'}</div>
            </div>
            <label style="display:inline-flex; align-items:center; cursor:pointer; gap:8px;">
              <input type="checkbox" id="dock-magnify-toggle" ${curDockMagnify ? 'checked' : ''} onchange="DokuSettings.toggleDockMagnify(this.checked)" style="width:18px; height:18px; cursor:pointer;">
              <span style="font-size:12px; font-weight:600;">${curDockMagnify ? (isId ? 'Aktif' : 'Enabled') : (isId ? 'Nonaktif' : 'Disabled')}</span>
            </label>
          </div>

          <!-- Posisi Dock -->
          <div style="display:flex; align-items:center; justify-content:space-between; padding-top:14px; border-top:1px solid var(--window-border);">
            <div>
              <div style="font-size:13px; font-weight:600;">${isId ? 'Posisi di Layar' : 'Position on Screen'}</div>
              <div style="font-size:11px; color:var(--text-muted);">${isId ? 'Pilih penempatan Dock di layar desktop.' : 'Choose dock alignment on screen.'}</div>
            </div>
            <div style="display:flex; gap:6px;">
              <button class="mac-btn ${curDockPos === 'bottom' ? 'primary' : ''}" onclick="DokuSettings.applyDockPosition('bottom')" style="font-size:11px; padding:3px 10px;">⬇️ ${isId ? 'Bawah' : 'Bottom'}</button>
              <button class="mac-btn ${curDockPos === 'left' ? 'primary' : ''}" onclick="DokuSettings.applyDockPosition('left')" style="font-size:11px; padding:3px 10px;">⬅️ ${isId ? 'Kiri' : 'Left'}</button>
              <button class="mac-btn ${curDockPos === 'right' ? 'primary' : ''}" onclick="DokuSettings.applyDockPosition('right')" style="font-size:11px; padding:3px 10px;">➡️ ${isId ? 'Kanan' : 'Right'}</button>
            </div>
          </div>
        </div>

        <!-- 2. Theme & Accent Color Card -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:14px; padding:20px; margin-bottom:20px;">
          <h3 style="font-size:15px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:8px;">
            <span>🎨</span> ${isId ? 'Tema & Warna Aksen (Accent Color)' : 'Theme & Accent Color'}
          </h3>
          <p style="font-size:12px; color:var(--text-muted); margin-bottom:16px;">
            ${isId ? 'Ubah mode terang / gelap dan warna tombol serta seleksi di seluruh sistem.' : 'Choose your preferred desktop light/dark theme and accent color.'}
          </p>

          <!-- Light / Dark Selector -->
          <div style="display:flex; gap:14px; margin-bottom:20px;">
            <div onclick="DokuSettings.applyThemeMode('light')" style="flex:1; border:2px solid ${curTheme === 'light' ? 'var(--accent-color)' : 'var(--window-border)'}; border-radius:10px; padding:12px; cursor:pointer; background:#ffffff; color:#1d1d1f; text-align:center; box-shadow:0 4px 12px rgba(0,0,0,0.06);">
              <div style="font-size:28px; margin-bottom:6px;">☀️</div>
              <div style="font-weight:700; font-size:13px;">Light Mode</div>
              <div style="font-size:11px; opacity:0.7;">${isId ? 'Tampilan Terang Bersih' : 'Clean & bright'}</div>
            </div>
            <div onclick="DokuSettings.applyThemeMode('dark')" style="flex:1; border:2px solid ${curTheme === 'dark' ? 'var(--accent-color)' : 'var(--window-border)'}; border-radius:10px; padding:12px; cursor:pointer; background:#1e1e24; color:#f5f5f7; text-align:center; box-shadow:0 4px 12px rgba(0,0,0,0.3);">
              <div style="font-size:28px; margin-bottom:6px;">🌙</div>
              <div style="font-weight:700; font-size:13px;">Dark Mode</div>
              <div style="font-size:11px; opacity:0.7;">${isId ? 'Aesthetic Gelap Sonoma' : 'Deep obsidian macOS'}</div>
            </div>
          </div>

          <!-- Accent Color Swatches -->
          <div>
            <label style="font-size:12px; font-weight:700; display:block; margin-bottom:10px;">
              ${isId ? 'Warna Aksen Tombol & Seleksi:' : 'Accent Color:'}
            </label>
            <div style="display:flex; gap:12px; align-items:center; flex-wrap:wrap;">
              ${accentPalettes.map(p => `
                <div onclick="DokuSettings.applyAccentColor('${p.color}')" title="${p.name}" style="display:flex; flex-direction:column; align-items:center; gap:4px; cursor:pointer;">
                  <div style="width:30px; height:30px; border-radius:50%; background:${p.color}; border:3px solid ${(curAccent.toLowerCase() === p.color.toLowerCase()) ? '#fff' : 'transparent'}; box-shadow:0 2px 8px rgba(0,0,0,0.25); transform:${(curAccent.toLowerCase() === p.color.toLowerCase()) ? 'scale(1.15)' : 'none'}; transition:transform 0.15s;"></div>
                  <span style="font-size:10px; color:var(--text-muted);">${p.name}</span>
                </div>
              `).join('')}
              <div style="margin-left:8px; display:flex; align-items:center; gap:6px;">
                <input type="color" value="${curAccent}" onchange="DokuSettings.applyAccentColor(this.value)" style="width:30px; height:30px; border-radius:50%; border:none; cursor:pointer; background:none;">
                <span style="font-size:11px; color:var(--text-muted);">${isId ? 'Kustom' : 'Custom'}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- 3. Desktop Wallpaper Quick Access -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:14px; padding:20px;">
          <div style="display:flex; justify-content:space-between; align-items:center;">
            <div>
              <h3 style="font-size:15px; font-weight:700; display:flex; align-items:center; gap:8px;">
                <span>🖼️</span> ${isId ? 'Wallpaper Desktop macOS' : 'Desktop Wallpaper'}
              </h3>
              <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
                ${isId ? 'Ganti latar belakang desktop dengan pemandangan Sonoma, Ventura, atau Cyber Obsidian.' : 'Choose your favorite desktop wallpaper background.'}
              </p>
            </div>
            <button class="mac-btn primary" onclick="DokuHub.openApp('wallpaper')">
              🖼️ ${isId ? 'Buka Galeri Wallpaper' : 'Change Wallpaper...'}
            </button>
          </div>
        </div>
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     Tenant Management Tab (SuperAdmin Only)
     ------------------------------------------------------------------------ */
  renderTenantsTab() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    return `
      <div>
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:20px;">
          <div>
            <h2 style="font-size:20px; font-weight:700;">🏢 ${isId ? 'Manajemen Organisasi & Tenant' : 'Organization & Tenant Management'}</h2>
            <p style="font-size:12px; color:var(--text-muted); margin-top:4px;">
              ${isId 
                ? 'Kelola organisasi berdaulat, isolasi data multitenant, dan alokasi pengguna pada sistem DokuHub.' 
                : 'Manage isolated sovereign tenants, multitenant boundaries, and organization plan tiers.'}
            </p>
          </div>
          <button class="mac-btn primary" onclick="DokuSettings.openAddTenantModal()">
            ✨ + ${isId ? 'Tambah Tenant' : 'New Tenant'}
          </button>
        </div>

        <!-- Tenant Table -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; overflow:hidden;">
          <table style="width:100%; border-collapse:collapse; font-size:13px;">
            <thead>
              <tr style="border-bottom:1px solid var(--window-border); text-align:left; color:var(--text-muted); font-size:11px; background:rgba(125,125,125,0.04);">
                <th style="padding:10px 14px;">${isId ? 'ORGANISASI / TENANT' : 'ORGANIZATION / TENANT'}</th>
                <th style="padding:10px 14px;">ID / SLUG</th>
                <th style="padding:10px 14px;">${isId ? 'PAKET' : 'TIER PLAN'}</th>
                <th style="padding:10px 14px;">${isId ? 'PENGGUNA' : 'USERS'}</th>
                <th style="padding:10px 14px;">${isId ? 'BERKAS' : 'FILES'}</th>
                <th style="padding:10px 14px; text-align:right;">${isId ? 'AKSI' : 'ACTIONS'}</th>
              </tr>
            </thead>
            <tbody>
              ${this.tenants.map(t => {
                const isCurrent = (t.id === (DokuHub.currentUser ? DokuHub.currentUser.tenant_id : ''));
                return `
                  <tr style="border-bottom:1px solid var(--window-border); ${isCurrent ? 'background:rgba(0,122,255,0.05);' : ''}">
                    <td style="padding:12px 14px;">
                      <div style="display:flex; align-items:center; gap:10px;">
                        <div style="width:32px; height:32px; border-radius:8px; background:linear-gradient(135deg, #007aff, #10b981); color:#fff; display:flex; align-items:center; justify-content:center; font-size:16px;">
                          🏢
                        </div>
                        <div>
                          <div style="font-weight:600; font-size:13px;">
                            ${t.name}
                            ${isCurrent ? '<span style="font-size:10px; color:var(--accent-color); font-weight:700; margin-left:6px;">(ACTIVE)</span>' : ''}
                          </div>
                          <div style="font-size:11px; color:var(--text-muted);">ID: ${t.id}</div>
                        </div>
                      </div>
                    </td>
                    <td style="padding:12px 14px;">
                      <code style="background:rgba(125,125,125,0.12); padding:2px 6px; border-radius:4px; font-size:11px;">${t.slug}</code>
                    </td>
                    <td style="padding:12px 14px;">
                      <span style="padding:3px 9px; border-radius:12px; font-size:11px; font-weight:600; background:rgba(16,185,129,0.15); color:#10b981;">
                        ${t.plan || 'Standard'}
                      </span>
                    </td>
                    <td style="padding:12px 14px;">
                      <span style="font-weight:600;">${t.user_count !== undefined ? t.user_count : '--'}</span>
                      <span style="font-size:11px; color:var(--text-muted); margin-left:2px;">user</span>
                    </td>
                    <td style="padding:12px 14px;">
                      <span style="font-weight:600;">${t.file_count !== undefined ? t.file_count : '--'}</span>
                      <span style="font-size:11px; color:var(--text-muted); margin-left:2px;">files</span>
                    </td>
                    <td style="padding:12px 14px; text-align:right;">
                      <div style="display:inline-flex; gap:6px;">
                        <button class="mac-btn" style="font-size:11px; padding:3px 10px;" onclick="DokuSettings.openEditTenantModal('${t.id}')">
                          ✏️ ${isId ? 'Edit' : 'Edit'}
                        </button>
                        ${!isCurrent ? `
                          <button class="mac-btn" style="font-size:11px; padding:3px 8px; color:#ef4444;" onclick="DokuSettings.deleteTenant('${t.id}')" title="${isId ? 'Hapus tenant' : 'Delete tenant'}">
                            🗑️
                          </button>
                        ` : ''}
                      </div>
                    </td>
                  </tr>
                `;
              }).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `;
  },

  renderRbacTab() {
    const cur = DokuHub.currentUser || (this.users.length > 0 ? this.users[0] : { name: 'Admin', email: '', role: 'SuperAdmin' });
    const curInitials = (cur.name || 'User').split(' ').map(n=>n[0]).join('').slice(0, 2).toUpperCase();
    const curAvatar = cur.avatar || (this.users.find(u => u.id === cur.id) ? this.users.find(u => u.id === cur.id).avatar : '') || curInitials;
    const curColor = cur.color || (this.users.find(u => u.id === cur.id) ? this.users.find(u => u.id === cur.id).color : '') || '#007aff';

    return `
      <div>
        <!-- Active User Profile Banner -->
        <div style="background:linear-gradient(135deg, rgba(0,122,255,0.12), rgba(16,185,129,0.08)); border:1px solid rgba(0,122,255,0.25); border-radius:12px; padding:18px 22px; margin-bottom:24px; display:flex; justify-content:space-between; align-items:center;">
          <div style="display:flex; align-items:center; gap:14px;">
            ${this.renderAvatarHtml(curAvatar, curColor, curInitials, 52)}
            <div>
              <div style="font-size:11px; font-weight:700; text-transform:uppercase; color:var(--accent-color);">Current Active Profile</div>
              <h2 style="font-size:19px; font-weight:700; margin:2px 0;">${cur.name}</h2>
              <div style="font-size:12px; color:var(--text-muted);">${cur.email} • Role: <strong>${cur.role}</strong> • Tenant: <strong>${cur.tenant || (DokuHub.activeTenant && DokuHub.activeTenant.name) || 'Workspace'}</strong></div>
            </div>
          </div>
          <button class="mac-btn primary" onclick="DokuSettings.showTab('profile')">✏️ Edit My Profile</button>
        </div>

        <!-- Tenant Context Header -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:14px 18px; margin-bottom:24px; display:flex; justify-content:space-between; align-items:center;">
          <div>
            <div style="font-size:11px; font-weight:700; text-transform:uppercase; color:var(--text-muted);">Active Workspace Organization / Tenant</div>
            <div style="font-size:16px; font-weight:700;">🏢 ${cur.tenant || DokuHub.activeTenant.name}</div>
            <div style="font-size:11px; color:var(--text-muted);">
              ${(cur.role === 'SuperAdmin') 
                ? 'Multi-tenant Authority: As SuperAdmin you can inspect and manage multiple organizations' 
                : 'Isolated Tenant Environment: You are bound strictly to your organization data'}
            </div>
          </div>
          <div>
            <span style="font-size:12px; font-weight:600; padding:4px 10px; background:rgba(0,122,255,0.1); border:1px solid rgba(0,122,255,0.3); border-radius:6px; color:var(--accent-color);">
              🔒 Dedicated Tenant Lock
            </span>
          </div>
        </div>

        <!-- Role Permissions Matrix Card -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:18px; margin-bottom:24px;">
          <h3 style="font-size:15px; font-weight:700; margin-bottom:12px;">RBAC Matrix (Role-Based Access Control)</h3>
          <table style="width:100%; border-collapse:collapse; font-size:12px;">
            <thead>
              <tr style="border-bottom:1px solid var(--window-border); text-align:left; color:var(--text-muted);">
                <th style="padding:8px 6px;">Privilege / Capability</th>
                <th style="padding:8px 6px;">SuperAdmin</th>
                <th style="padding:8px 6px;">Admin</th>
                <th style="padding:8px 6px;">Supervisor</th>
                <th style="padding:8px 6px;">Staff</th>
                <th style="padding:8px 6px;">Guest / External</th>
              </tr>
            </thead>
            <tbody>
              <tr style="border-bottom:1px solid var(--window-border);">
                <td style="padding:8px 6px; font-weight:600;">Manage Tenants & Users</td>
                <td style="color:#10b981;">✓ All Tenants</td>
                <td style="color:#10b981;">✓ Own Tenant</td>
                <td style="color:#ef4444;">✗</td>
                <td style="color:#ef4444;">✗</td>
                <td style="color:#ef4444;">✗</td>
              </tr>
              <tr style="border-bottom:1px solid var(--window-border);">
                <td style="padding:8px 6px; font-weight:600;">Approve Share Dokumen (Internal/External)</td>
                <td style="color:#10b981;">✓ All</td>
                <td style="color:#10b981;">✓ Tenant</td>
                <td style="color:#10b981;">✓ Bagian / Tim</td>
                <td style="color:#f59e0b;">⏳ Ajukan</td>
                <td style="color:#ef4444;">✗</td>
              </tr>
              <tr style="border-bottom:1px solid var(--window-border);">
                <td style="padding:8px 6px; font-weight:600;">Create Public Links (Tanpa Approval)</td>
                <td style="color:#10b981;">✓</td>
                <td style="color:#10b981;">✓</td>
                <td style="color:#10b981;">✓</td>
                <td style="color:#10b981;">✓</td>
                <td style="color:#ef4444;">✗</td>
              </tr>
              <tr>
                <td style="padding:8px 6px; font-weight:600;">Host WebRTC Video Conference</td>
                <td style="color:#10b981;">✓ Host</td>
                <td style="color:#10b981;">✓ Host</td>
                <td style="color:#10b981;">✓ Host</td>
                <td style="color:#10b981;">✓ Host</td>
                <td style="color:#10b981;">✓ Participant</td>
              </tr>
            </tbody>
          </table>
        </div>

        ${this.renderOrgHierarchyCard()}

        <!-- Users Management Table -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:18px;">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:14px;">
            <div>
              <h3 style="font-size:15px; font-weight:700;">Workspace Users (${this.users.length})</h3>
              <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">Each user maintains their own independent profile, credentials, department, supervisor, and isolated documents workspace.</p>
            </div>
            <button class="mac-btn primary" onclick="DokuSettings.openAddUserModal()">+ Add New User</button>
          </div>

          <table style="width:100%; border-collapse:collapse; font-size:13px;">
            <thead>
              <tr style="border-bottom:1px solid var(--window-border); text-align:left; color:var(--text-muted); font-size:11px;">
                <th style="padding:8px 6px;">USER</th>
                <th style="padding:8px 6px;">ROLE & HIRARKI</th>
                <th style="padding:8px 6px;">STORAGE & QUOTA</th>
                <th style="padding:8px 6px;">STATUS</th>
                <th style="padding:8px 6px; text-align:right;">ACTIONS</th>
              </tr>
            </thead>
            <tbody>
              ${this.users.map(u => {
                const isCurrent = (u.id === DokuHub.currentUser.id);
                const uInitials = (u.name || 'User').split(' ').map(n=>n[0]).join('').slice(0,2).toUpperCase();
                const uAvatar = (isCurrent && DokuHub.currentUser.avatar) ? DokuHub.currentUser.avatar : (u.avatar || uInitials);
                const uColor = (isCurrent && DokuHub.currentUser.color) ? DokuHub.currentUser.color : (u.color || '#007aff');
                const qMb = u.storage_quota_mb || 10240;
                const qLabel = (qMb >= 1024) ? `${(qMb / 1024).toFixed(qMb % 1024 === 0 ? 0 : 1)} GB` : `${Math.round(qMb)} MB`;
                const roleLabel = (u.role === 'Member' || u.role === 'Staff') ? 'Staff' : u.role;
                return `
                  <tr style="border-bottom:1px solid var(--window-border); ${isCurrent ? 'background:rgba(0,122,255,0.06);' : ''}">
                    <td style="padding:10px 6px;">
                      <div style="display:flex; align-items:center; gap:10px;">
                        ${this.renderAvatarHtml(uAvatar, uColor, uInitials, 32)}
                        <div>
                          <div style="font-weight:600;">${u.name} ${isCurrent ? '<span style="font-size:10px; color:var(--accent-color); font-weight:700;">(YOU)</span>' : ''}</div>
                          <div style="font-size:11px; color:var(--text-muted);">${u.email}</div>
                        </div>
                      </div>
                    </td>
                    <td style="padding:10px 6px;">
                      <span style="padding:2px 8px; border-radius:10px; font-size:11px; font-weight:600; background:${this.getRoleBadgeBg(u.role)}; color:${this.getRoleBadgeColor(u.role)};">
                        ${roleLabel}
                      </span>
                      ${u.department ? `<div style="font-size:11px; color:var(--text-main); margin-top:4px;">🏷️ ${u.department}</div>` : ''}
                      ${u.supervisor_name ? `<div style="font-size:10px; color:var(--text-muted); margin-top:2px;">↳ Spv: <strong>${u.supervisor_name}</strong></div>` : ''}
                    </td>
                    <td style="padding:10px 6px;">
                      <div style="font-size:12px; font-weight:600;">${(u.storage_used_mb || 0).toFixed(2)} MB / ${qLabel}</div>
                      <div style="width:110px; height:5px; background:rgba(125,125,125,0.2); border-radius:3px; overflow:hidden; margin-top:4px;">
                        <div style="width:${Math.min(100, u.storage_percent || 0)}%; height:100%; background:${(u.storage_percent || 0) > 85 ? '#ef4444' : '#10b981'};"></div>
                      </div>
                      <div style="font-size:10px; color:var(--text-muted); margin-top:2px;">${u.storage_percent || 0}% used</div>
                    </td>
                    <td style="padding:10px 6px;">
                      <span style="color:${u.status === 'suspended' ? '#ef4444' : '#10b981'}; font-size:12px;">
                        ● ${u.status === 'suspended' ? 'Suspended' : 'Active'}
                      </span>
                    </td>
                    <td style="padding:10px 6px; text-align:right;">
                      <div style="display:inline-flex; gap:6px;">
                        <button class="mac-btn" style="font-size:11px; padding:2px 8px;" onclick="DokuSettings.openEditUserModal('${u.id}')" title="Edit user">
                          ✏️ Edit
                        </button>
                        ${!isCurrent ? `
                          <button class="mac-btn" style="font-size:11px; padding:2px 8px; color:#ef4444;" onclick="DokuSettings.deleteUser('${u.id}')" title="Delete user">
                            🗑️
                          </button>
                        ` : ''}
                      </div>
                    </td>
                  </tr>
                `;
              }).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `;
  },

  renderOrgHierarchyCard() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const viewMode = this.orgViewMode || 'topology';
    const admins = this.users.filter(u => u.role === 'Admin' || u.role === 'SuperAdmin');
    const supervisors = this.users.filter(u => u.role === 'Supervisor');
    const staffs = this.users.filter(u => u.role === 'Staff' || u.role === 'Member');
    const unassignedStaff = staffs.filter(s => {
      if (s.supervisor_id && supervisors.some(spv => spv.id === s.supervisor_id)) return false;
      if (!s.supervisor_id && s.department && supervisors.some(spv => spv.department && spv.department.toLowerCase() === s.department.toLowerCase())) return false;
      return true;
    });

    const tenantName = (DokuHub.activeTenant && DokuHub.activeTenant.name) ? DokuHub.activeTenant.name : 'Tenant';

    return `
      <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:18px; margin-bottom:20px;">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:14px; flex-wrap:wrap; gap:10px;">
          <div>
            <h3 style="font-size:15px; font-weight:700;">🗺️ ${isId ? 'Topology Map Hirarki Tenant & Alur Approval' : 'Tenant Hierarchy Topology Map & Approval Flow'}</h3>
            <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
              ${isId ? 'Klik node user untuk mengubah Role, Bagian, atau Atasan (Supervisor). Garis putus-putus menunjukkan alur pengajuan approval share.' : 'Click any user node to edit Role, Department, or Supervisor. Flow lines illustrate the share approval path.'}
            </p>
          </div>
          <div style="display:flex; gap:6px; flex-wrap:wrap;">
            <button class="mac-btn ${viewMode === 'topology' ? 'primary' : ''}" style="font-size:11px; padding:4px 10px;" onclick="DokuSettings.orgViewMode='topology'; DokuSettings.render();">🗺️ Topology Map</button>
            <button class="mac-btn ${viewMode === 'list' ? 'primary' : ''}" style="font-size:11px; padding:4px 10px;" onclick="DokuSettings.orgViewMode='list'; DokuSettings.render();">📋 List View</button>
            <button class="mac-btn" style="font-size:11px; padding:4px 10px;" onclick="DokuSettings.showTab('approvals')">✅ ${isId ? 'Antrean Approval' : 'Approval Queue'}</button>
          </div>
        </div>

        ${viewMode === 'topology'
          ? this.renderOrgTopologySvg(tenantName, admins, supervisors, staffs, unassignedStaff)
          : this.renderOrgListView(admins, supervisors, staffs, unassignedStaff)}
      </div>
    `;
  },

  renderOrgTopologySvg(tenantName, admins, supervisors, staffs, unassignedStaff) {
    const esc = (s) => String(s || '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    const trunc = (s, max = 16) => {
      const str = String(s || '');
      return str.length > max ? str.slice(0, max - 1) + '…' : str;
    };

    // Build branches under Admin Tenant
    const branches = supervisors.map(spv => {
      const members = staffs.filter(s =>
        s.supervisor_id === spv.id ||
        (!s.supervisor_id && spv.department && s.department && s.department.toLowerCase() === spv.department.toLowerCase())
      );
      return {
        type: 'supervisor',
        id: spv.id,
        title: spv.name,
        subtitle: spv.department ? `Spv • ${spv.department}` : 'Supervisor',
        color: '#8b5cf6',
        members
      };
    });

    if (unassignedStaff.length > 0) {
      branches.push({
        type: 'unassigned',
        id: '',
        title: 'Direct to Admin',
        subtitle: 'Belum Ada Supervisor',
        color: '#f59e0b',
        members: unassignedStaff
      });
    }

    // If no supervisors and no staff yet, show an illustrative placeholder branch
    const isEmptyHierarchy = branches.length === 0;
    if (isEmptyHierarchy) {
      branches.push(
        { type: 'placeholder', id: '', title: 'Supervisor Bagian A', subtitle: 'Contoh Node', color: '#8b5cf6', members: [{ id: '', name: 'Staff Bagian A', department: 'Bagian A', placeholder: true }] },
        { type: 'placeholder', id: '', title: 'Supervisor Bagian B', subtitle: 'Contoh Node', color: '#8b5cf6', members: [{ id: '', name: 'Staff Bagian B', department: 'Bagian B', placeholder: true }] }
      );
    }

    // Layout geometry
    const leafW = 148;
    const leafGap = 16;
    const branchGap = 36;
    const nodeH = 54;
    const rootY = 26;
    const spvY = 142;
    const staffY = 262;
    const svgH = 352;

    const branchWidths = branches.map(b => {
      const count = Math.max(1, b.members.length);
      return Math.max(186, count * leafW + (count - 1) * leafGap);
    });
    const totalW = Math.max(660, branchWidths.reduce((a, b) => a + b, 0) + Math.max(0, branches.length - 1) * branchGap + 60);
    const rootCenterX = totalW / 2;
    const rootW = 230;
    const rootX = rootCenterX - rootW / 2;

    const primaryAdmin = admins[0] || null;
    const adminLabel = admins.length > 1
      ? `${admins[0].name} (+${admins.length - 1})`
      : (primaryAdmin ? primaryAdmin.name : 'Admin Tenant');

    let edgesSvg = '';
    let nodesSvg = '';

    // Root Node (Admin Tenant)
    const rootClick = primaryAdmin ? `onclick="DokuSettings.openEditUserModal('${primaryAdmin.id}')" style="cursor:pointer;"` : '';
    nodesSvg += `
      <g ${rootClick}>
        <rect x="${rootX}" y="${rootY}" width="${rootW}" height="${nodeH}" rx="14" fill="rgba(14,165,233,0.16)" stroke="#0ea5e9" stroke-width="2" filter="url(#topo-glow)"/>
        <circle cx="${rootX + 24}" cy="${rootY + 27}" r="14" fill="#0ea5e9"/>
        <text x="${rootX + 24}" y="${rootY + 32}" text-anchor="middle" font-size="14">👑</text>
        <text x="${rootX + 46}" y="${rootY + 23}" fill="currentColor" font-size="12" font-weight="700">${esc(trunc(adminLabel, 22))}</text>
        <text x="${rootX + 46}" y="${rootY + 39}" fill="#0284c7" font-size="10" font-weight="600">ADMIN TENANT • ${esc(trunc(tenantName, 18))}</text>
      </g>
    `;

    let cursorX = (totalW - (branchWidths.reduce((a, b) => a + b, 0) + Math.max(0, branches.length - 1) * branchGap)) / 2;

    branches.forEach((b, bIdx) => {
      const bw = branchWidths[bIdx];
      const spvCenterX = cursorX + bw / 2;
      const spvW = 186;
      const spvX = spvCenterX - spvW / 2;

      // Curved Bezier Edge: Admin Tenant -> Supervisor
      const p1x = rootCenterX;
      const p1y = rootY + nodeH;
      const p2x = spvCenterX;
      const p2y = spvY;
      const midY1 = (p1y + p2y) / 2;
      const dash = (b.type === 'unassigned' || b.type === 'placeholder') ? 'stroke-dasharray="5 4"' : '';

      edgesSvg += `
        <path d="M ${p1x} ${p1y} C ${p1x} ${midY1}, ${p2x} ${midY1}, ${p2x} ${p2y}"
              fill="none" stroke="${b.color}" stroke-width="2" stroke-opacity="0.7" ${dash}/>
        <circle r="3.5" fill="${b.color}">
          <animateMotion dur="${2.8 + bIdx * 0.4}s" repeatCount="indefinite"
            path="M ${p2x} ${p2y} C ${p2x} ${midY1}, ${p1x} ${midY1}, ${p1x} ${p1y}"/>
        </circle>
      `;

      // Supervisor Node
      const spvClick = b.id ? `onclick="DokuSettings.openEditUserModal('${b.id}')" style="cursor:pointer;"` : '';
      const spvOpacity = b.type === 'placeholder' ? 'opacity="0.65"' : '';
      nodesSvg += `
        <g ${spvClick} ${spvOpacity}>
          <rect x="${spvX}" y="${spvY}" width="${spvW}" height="${nodeH}" rx="12" fill="${b.type === 'unassigned' ? 'rgba(245,158,11,0.14)' : 'rgba(139,92,246,0.14)'}" stroke="${b.color}" stroke-width="1.8" ${dash}/>
          <circle cx="${spvX + 22}" cy="${spvY + 27}" r="13" fill="${b.color}"/>
          <text x="${spvX + 22}" y="${spvY + 32}" text-anchor="middle" font-size="13">${b.type === 'unassigned' ? '⚡' : '🛡️'}</text>
          <text x="${spvX + 42}" y="${spvY + 23}" fill="currentColor" font-size="11.5" font-weight="700">${esc(trunc(b.title, 17))}</text>
          <text x="${spvX + 42}" y="${spvY + 39}" fill="${b.color}" font-size="10" font-weight="600">${esc(trunc(b.subtitle, 20))}</text>
        </g>
      `;

      // Staff Leaf Nodes
      const members = b.members || [];
      if (members.length > 0) {
        const totalLeavesW = members.length * leafW + (members.length - 1) * leafGap;
        let leafStartX = spvCenterX - totalLeavesW / 2;

        members.forEach((st, sIdx) => {
          const stCenterX = leafStartX + leafW / 2;
          const s1x = spvCenterX;
          const s1y = spvY + nodeH;
          const s2x = stCenterX;
          const s2y = staffY;
          const midY2 = (s1y + s2y) / 2;
          const leafDash = st.placeholder ? 'stroke-dasharray="4 4"' : '';

          edgesSvg += `
            <path d="M ${s1x} ${s1y} C ${s1x} ${midY2}, ${s2x} ${midY2}, ${s2x} ${s2y}"
                  fill="none" stroke="#10b981" stroke-width="1.8" stroke-opacity="0.7" ${leafDash}/>
            <circle r="3" fill="#10b981">
              <animateMotion dur="${2.2 + sIdx * 0.35}s" repeatCount="indefinite"
                path="M ${s2x} ${s2y} C ${s2x} ${midY2}, ${s1x} ${midY2}, ${s1x} ${s1y}"/>
            </circle>
          `;

          const stClick = st.id ? `onclick="DokuSettings.openEditUserModal('${st.id}')" style="cursor:pointer;"` : '';
          const stOpacity = st.placeholder ? 'opacity="0.6"' : '';
          nodesSvg += `
            <g ${stClick} ${stOpacity}>
              <rect x="${leafStartX}" y="${staffY}" width="${leafW}" height="50" rx="11" fill="rgba(16,185,129,0.13)" stroke="#10b981" stroke-width="1.6" ${leafDash}/>
              <circle cx="${leafStartX + 20}" cy="${staffY + 25}" r="11" fill="#10b981"/>
              <text x="${leafStartX + 20}" y="${staffY + 29}" text-anchor="middle" font-size="11">👤</text>
              <text x="${leafStartX + 37}" y="${staffY + 22}" fill="currentColor" font-size="11" font-weight="700">${esc(trunc(st.name, 13))}</text>
              <text x="${leafStartX + 37}" y="${staffY + 37}" fill="#059669" font-size="9.5" font-weight="600">${esc(trunc(st.department || 'Staff', 14))}</text>
            </g>
          `;

          leafStartX += leafW + leafGap;
        });
      }

      cursorX += bw + branchGap;
    });

    return `
      <div style="border-radius:12px; border:1px solid var(--window-border); background-color:var(--window-bg); background-image:radial-gradient(rgba(128,128,128,0.22) 1px, transparent 1px); background-size:18px 18px; overflow-x:auto; padding:8px;">
        <svg viewBox="0 0 ${totalW} ${svgH}" style="width:100%; min-width:${Math.min(totalW, 640)}px; height:${svgH}px; display:block; color:var(--text-main);">
          <defs>
            <filter id="topo-glow" x="-20%" y="-20%" width="140%" height="140%">
              <feDropShadow dx="0" dy="4" stdDeviation="4" flood-color="#0ea5e9" flood-opacity="0.22"/>
            </filter>
          </defs>
          ${edgesSvg}
          ${nodesSvg}
        </svg>
      </div>
      <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:10px; margin-top:10px; font-size:11px; color:var(--text-muted);">
        <div style="display:flex; gap:14px; flex-wrap:wrap;">
          <span><span style="display:inline-block; width:9px; height:9px; border-radius:50%; background:#0ea5e9; margin-right:4px;"></span>Admin Tenant</span>
          <span><span style="display:inline-block; width:9px; height:9px; border-radius:50%; background:#8b5cf6; margin-right:4px;"></span>Supervisor (Approver)</span>
          <span><span style="display:inline-block; width:9px; height:9px; border-radius:50%; background:#10b981; margin-right:4px;"></span>Staff (Pengaju Share)</span>
          <span><span style="display:inline-block; width:9px; height:9px; border-radius:50%; background:#f59e0b; margin-right:4px;"></span>Belum Ada Supervisor</span>
        </div>
        <div>${isEmptyHierarchy ? '💡 <em>Contoh topologi — tambahkan user Supervisor & Staff untuk membentuk topologi nyata.</em>' : '● Titik bergerak menunjukkan arah pengajuan Approval (Staff → Supervisor → Admin)'}</div>
      </div>
    `;
  },

  renderOrgListView(admins, supervisors, staffs, unassignedStaff) {
    return `
      <div style="display:flex; flex-direction:column; gap:12px;">
        <div style="padding:10px 14px; border-radius:10px; background:rgba(14,165,233,0.08); border:1px solid rgba(14,165,233,0.25);">
          <div style="font-size:11px; font-weight:700; color:#0284c7; text-transform:uppercase; margin-bottom:6px;">Level 1 — Admin Tenant</div>
          <div style="display:flex; flex-wrap:wrap; gap:8px;">
            ${admins.length ? admins.map(a => `
              <span onclick="DokuSettings.openEditUserModal('${a.id}')" style="cursor:pointer; display:inline-flex; align-items:center; gap:6px; padding:4px 10px; border-radius:16px; background:var(--window-bg); border:1px solid var(--window-border); font-size:12px; font-weight:600;">
                👑 ${a.name} <span style="font-size:10px; color:var(--text-muted);">(${a.role})</span>
              </span>
            `).join('') : '<span style="font-size:12px; color:var(--text-muted);">Belum ada Admin Tenant</span>'}
          </div>
        </div>
        <div style="display:grid; grid-template-columns:repeat(auto-fill, minmax(260px, 1fr)); gap:12px; padding-left:16px; border-left:2px dashed var(--window-border);">
          ${supervisors.length ? supervisors.map(spv => {
            const myStaff = staffs.filter(s => s.supervisor_id === spv.id || (!s.supervisor_id && spv.department && s.department && s.department.toLowerCase() === spv.department.toLowerCase()));
            return `
              <div style="padding:12px; border-radius:10px; background:rgba(139,92,246,0.07); border:1px solid rgba(139,92,246,0.28);">
                <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px; cursor:pointer;" onclick="DokuSettings.openEditUserModal('${spv.id}')">
                  <div>
                    <div style="font-size:11px; font-weight:700; color:#7c3aed; text-transform:uppercase;">Supervisor ${spv.department ? '— ' + spv.department : ''}</div>
                    <div style="font-size:13px; font-weight:700; margin-top:2px;">🛡️ ${spv.name}</div>
                  </div>
                  <span style="font-size:10px; padding:2px 7px; border-radius:10px; background:rgba(139,92,246,0.18); color:#7c3aed; font-weight:700;">${myStaff.length} Staff</span>
                </div>
                <div style="padding-left:10px; border-left:2px solid rgba(16,185,129,0.4); display:flex; flex-direction:column; gap:4px;">
                  ${myStaff.length ? myStaff.map(st => `
                    <div onclick="DokuSettings.openEditUserModal('${st.id}')" style="cursor:pointer; font-size:12px; display:flex; align-items:center; justify-content:space-between; padding:3px 6px; border-radius:6px; background:var(--window-bg);">
                      <span>👤 ${st.name}</span>
                      <span style="font-size:10px; color:var(--text-muted);">${st.department || 'Staff'}</span>
                    </div>
                  `).join('') : `<div style="font-size:11px; color:var(--text-muted); font-style:italic;">Belum ada Staff di bawah supervisor ini</div>`}
                </div>
              </div>
            `;
          }).join('') : `
            <div style="padding:12px; border-radius:10px; background:rgba(245,158,11,0.08); border:1px solid rgba(245,158,11,0.28); font-size:12px; color:var(--text-muted); grid-column:1/-1;">
              💡 Belum ada akun <strong>Supervisor</strong>. Klik <strong>+ Add New User</strong> atau <strong>Edit</strong> user untuk menetapkan Role <code>Supervisor</code> (mis. Bagian A / Bagian B) lalu tautkan <code>Staff</code> ke Supervisor-nya.
            </div>
          `}
        </div>
        ${unassignedStaff.length ? `
          <div style="padding:10px 14px; border-radius:10px; background:rgba(16,185,129,0.06); border:1px solid rgba(16,185,129,0.22);">
            <div style="font-size:11px; font-weight:700; color:#059669; margin-bottom:6px;">Staff Belum Ditautkan ke Supervisor (Approval jatuh ke Admin Tenant)</div>
            <div style="display:flex; flex-wrap:wrap; gap:6px;">
              ${unassignedStaff.map(st => `<span onclick="DokuSettings.openEditUserModal('${st.id}')" style="cursor:pointer; padding:3px 8px; border-radius:12px; font-size:11px; background:var(--window-bg); border:1px solid var(--window-border);">👤 ${st.name} ${st.department ? '(' + st.department + ')' : ''}</span>`).join('')}
            </div>
          </div>
        ` : ''}
      </div>
    `;
  },

  getRoleBadgeBg(role) {
    switch (role) {
      case 'SuperAdmin': return 'rgba(239, 68, 68, 0.15)';
      case 'Admin': return 'rgba(14, 165, 233, 0.15)';
      case 'Supervisor': return 'rgba(139, 92, 246, 0.16)';
      case 'Staff':
      case 'Member': return 'rgba(16, 185, 129, 0.15)';
      default: return 'rgba(107, 114, 128, 0.15)';
    }
  },

  getRoleBadgeColor(role) {
    switch (role) {
      case 'SuperAdmin': return '#ef4444';
      case 'Admin': return '#0284c7';
      case 'Supervisor': return '#7c3aed';
      case 'Staff':
      case 'Member': return '#059669';
      default: return '#4b5563';
    }
  },

  /* ------------------------------------------------------------------------
     Tenant CRUD Methods (SuperAdmin Only)
     ------------------------------------------------------------------------ */
  openAddTenantModal() {
    const modal = document.getElementById('modal-add-tenant');
    if (modal) modal.style.display = 'flex';
    const nameInput = document.getElementById('add-tenant-name');
    if (nameInput) {
      nameInput.value = '';
      nameInput.focus();
    }
    const slugInput = document.getElementById('add-tenant-slug');
    if (slugInput) slugInput.value = '';
  },

  closeAddTenantModal() {
    const modal = document.getElementById('modal-add-tenant');
    if (modal) modal.style.display = 'none';
  },

  async submitAddTenant() {
    const name = document.getElementById('add-tenant-name').value.trim();
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const slug = document.getElementById('add-tenant-slug').value.trim();
    const plan = document.getElementById('add-tenant-plan').value;

    if (!name) {
      alert(isId ? "Nama organisasi wajib diisi." : "Organization name is required.");
      return;
    }

    try {
      const res = await fetch('/api/tenants', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, slug, plan })
      });
      const data = await res.json();
      if (res.ok && data.success) {
        this.closeAddTenantModal();
        await this.loadAllData();
        this.render();
        alert(isId ? `Organisasi "${name}" berhasil dibuat!` : `Tenant "${name}" created successfully!`);
      } else {
        alert((isId ? "Gagal membuat organisasi: " : "Failed to create tenant: ") + (data.error || (isId ? 'Terjadi kesalahan' : 'Unknown error')));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  openEditTenantModal(tenantId) {
    this.editingTenantId = tenantId;
    const t = this.tenants.find(x => x.id === tenantId);
    if (!t) return;

    const modal = document.getElementById('modal-edit-tenant');
    if (modal) modal.style.display = 'flex';

    document.getElementById('edit-tenant-name').value = t.name || '';
    document.getElementById('edit-tenant-slug').value = t.slug || '';
    document.getElementById('edit-tenant-plan').value = t.plan || 'Enterprise Plus';
  },

  closeEditTenantModal() {
    const modal = document.getElementById('modal-edit-tenant');
    if (modal) modal.style.display = 'none';
    this.editingTenantId = null;
  },

  async submitEditTenant() {
    if (!this.editingTenantId) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const name = document.getElementById('edit-tenant-name').value.trim();
    const slug = document.getElementById('edit-tenant-slug').value.trim();
    const plan = document.getElementById('edit-tenant-plan').value;

    if (!name) {
      alert(isId ? "Nama organisasi wajib diisi." : "Organization name is required.");
      return;
    }

    try {
      const res = await fetch(`/api/tenants/${this.editingTenantId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, slug, plan })
      });
      const data = await res.json();
      if (res.ok && data.success) {
        this.closeEditTenantModal();
        await this.loadAllData();
        this.render();
        alert(isId ? `Organisasi "${name}" berhasil diperbarui!` : `Tenant "${name}" updated successfully!`);
      } else {
        alert((isId ? "Gagal memperbarui organisasi: " : "Failed to update tenant: ") + (data.error || (isId ? 'Terjadi kesalahan' : 'Unknown error')));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  async deleteTenant(tenantId) {
    const t = this.tenants.find(x => x.id === tenantId);
    if (!t) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    const confirmMsg = isId 
      ? `Hapus permanen organisasi "${t.name}"? Pastikan tidak ada data atau pengguna aktif yang terhubung.`
      : `Permanently delete tenant "${t.name}"? Ensure no active users or data are linked.`;
    if (!confirm(confirmMsg)) return;

    try {
      const res = await fetch(`/api/tenants/${tenantId}`, { method: 'DELETE' });
      const data = await res.json();
      if (res.ok && data.success) {
        await this.loadAllData();
        this.render();
        alert(isId ? `Organisasi "${t.name}" berhasil dihapus.` : `Tenant "${t.name}" successfully deleted.`);
      } else {
        alert((isId ? "Gagal menghapus organisasi: " : "Failed to delete tenant: ") + (data.error || (isId ? 'Terjadi kesalahan' : 'Unknown error')));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  /* ------------------------------------------------------------------------
     User CRUD: Add User
     ------------------------------------------------------------------------ */
  openAddUserModal() {
    const modal = document.getElementById('modal-add-user');
    if (modal) modal.style.display = 'flex';
    const nameInput = document.getElementById('add-user-name');
    if (nameInput) {
      nameInput.value = '';
      nameInput.focus();
    }
  },

  closeAddUserModal() {
    const modal = document.getElementById('modal-add-user');
    if (modal) modal.style.display = 'none';
  },

  async submitAddUser() {
    const name = document.getElementById('add-user-name').value.trim();
    const email = document.getElementById('add-user-email').value.trim();
    const role = document.getElementById('add-user-role').value;
    const deptEl = document.getElementById('add-user-dept');
    const supEl = document.getElementById('add-user-supervisor');
    const department = deptEl ? deptEl.value.trim() : '';
    const supervisorId = supEl ? supEl.value : '';
    const pass = document.getElementById('add-user-pass').value;
    const tenantSelect = document.getElementById('add-user-tenant');
    const tenantId = tenantSelect ? tenantSelect.value : DokuHub.activeTenant.id;
    const quota = this.getQuotaBytesValue('add');

    if (!name || !email) {
      alert("Name and email are required.");
      return;
    }

    try {
      const res = await fetch('/api/users', {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'X-Tenant-Id': DokuHub.activeTenant.id,
          'X-User-Id': DokuHub.currentUser ? DokuHub.currentUser.id : ''
        },
        body: JSON.stringify({
          name,
          email,
          role,
          department,
          supervisor_id: supervisorId,
          password: pass || 'password123',
          tenant_id: tenantId,
          storage_quota_bytes: quota,
          quota_bytes: quota
        })
      });
      const data = await res.json();
      if (data.success) {
        this.closeAddUserModal();
        await this.loadAllData();
        this.render();
        const gbLabel = (parseInt(quota) / (1024*1024*1024)).toFixed(0);
        alert(`User "${name}" successfully created with ${gbLabel} GB quota!`);
      } else {
        alert("Failed to create user: " + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  /* ------------------------------------------------------------------------
     User CRUD: Edit User
     ------------------------------------------------------------------------ */
  openEditUserModal(userId) {
    this.editingUserId = userId;
    const user = this.users.find(u => u.id === userId) || (userId === DokuHub.currentUser.id ? DokuHub.currentUser : null);
    if (!user) return;

    const modal = document.getElementById('modal-edit-user');
    if (modal) modal.style.display = 'flex';

    const isCurrent = (user.id === DokuHub.currentUser.id);
    const uInitials = (user.name || 'User').split(' ').map(n=>n[0]).join('').slice(0,2).toUpperCase();
    const uAvatar = (isCurrent && DokuHub.currentUser.avatar) ? DokuHub.currentUser.avatar : (user.avatar || uInitials);
    const uColor = (isCurrent && DokuHub.currentUser.color) ? DokuHub.currentUser.color : (user.color || '#007aff');

    const badgeEl = document.getElementById('edit-user-avatar-badge');
    if (badgeEl) {
      badgeEl.innerHTML = `
        ${this.renderAvatarHtml(uAvatar, uColor, uInitials, 38)}
        <div>
          <div style="font-size:14px; font-weight:700;">${user.name} ${isCurrent ? '<span style="font-size:10px; color:var(--accent-color); font-weight:700;">(YOU)</span>' : ''}</div>
          <div style="font-size:11px; color:var(--text-muted);">${user.email} • ID: <code>${user.id}</code></div>
        </div>
      `;
    }

    document.getElementById('edit-user-name').value = user.name || '';
    document.getElementById('edit-user-email').value = user.email || '';
    const mappedRole = (user.role === 'Member') ? 'Staff' : (user.role || 'Staff');
    document.getElementById('edit-user-role').value = mappedRole;
    document.getElementById('edit-user-status').value = user.status || 'active';

    const editDeptEl = document.getElementById('edit-user-dept');
    if (editDeptEl) editDeptEl.value = user.department || '';
    const editSupEl = document.getElementById('edit-user-supervisor');
    if (editSupEl) editSupEl.value = user.supervisor_id || '';

    // Set tenant select in edit user modal
    const tenantEl = document.getElementById('edit-user-tenant');
    if (tenantEl) {
      tenantEl.innerHTML = this.tenants.map(t => 
        `<option value="${t.id}" ${(t.id === (user.tenant_id || user.tenant)) ? 'selected' : ''}>🏢 ${t.name} (${t.slug})</option>`
      ).join('');
      tenantEl.value = user.tenant_id || (this.tenants[0] ? this.tenants[0].id : '');
    }

    const userQuota = String(user.storage_quota_bytes || user.quota_bytes || '10737418240');
    const quotaSelect = document.getElementById('edit-user-quota');
    const customWrap = document.getElementById('edit-user-quota-custom-wrap');
    const customInput = document.getElementById('edit-user-quota-custom');

    const standardValues = ['1073741824', '5368709120', '10737418240', '21474836480', '32212254720'];
    if (standardValues.includes(userQuota)) {
      if (quotaSelect) quotaSelect.value = userQuota;
      if (customWrap) customWrap.style.display = 'none';
    } else {
      if (quotaSelect) quotaSelect.value = 'custom';
      if (customWrap) customWrap.style.display = 'block';
      const gbVal = Math.round(parseInt(userQuota) / (1024 * 1024 * 1024));
      if (customInput) customInput.value = gbVal || 10;
    }

    document.getElementById('edit-user-pass').value = '';
  },

  closeEditUserModal() {
    const modal = document.getElementById('modal-edit-user');
    if (modal) modal.style.display = 'none';
    this.editingUserId = null;
  },

  async submitEditUser() {
    if (!this.editingUserId) return;
    const name = document.getElementById('edit-user-name').value.trim();
    const email = document.getElementById('edit-user-email').value.trim();
    const role = document.getElementById('edit-user-role').value;
    const status = document.getElementById('edit-user-status').value;
    const deptEl = document.getElementById('edit-user-dept');
    const supEl = document.getElementById('edit-user-supervisor');
    const department = deptEl ? deptEl.value.trim() : '';
    const supervisorId = supEl ? supEl.value : '';
    const pass = document.getElementById('edit-user-pass').value;
    const quota = this.getQuotaBytesValue('edit');
    const tenantEl = document.getElementById('edit-user-tenant');
    const tenantId = tenantEl ? tenantEl.value : undefined;

    if (!name || !email) {
      alert("Name and email cannot be empty.");
      return;
    }

    try {
      const payload = {
        name,
        email,
        role,
        status,
        department,
        supervisor_id: supervisorId,
        storage_quota_bytes: quota,
        quota_bytes: quota
      };
      if (pass) payload.password = pass;
      if (tenantId) payload.tenant_id = tenantId;

      const res = await fetch(`/api/users/${this.editingUserId}`, {
        method: 'PUT',
        headers: { 
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(payload)
      });
      const data = await res.json();
      if (data.success) {
        this.closeEditUserModal();
        await this.loadAllData();
        if (this.editingUserId === DokuHub.currentUser.id) {
          DokuHub.switchUser(data.user);
        }
        this.render();
        alert(`User profile "${name}" successfully updated!`);
      } else {
        alert("Failed to update user: " + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  /* ------------------------------------------------------------------------
     User CRUD: Delete User
     ------------------------------------------------------------------------ */
  async deleteUser(userId) {
    const target = this.users.find(u => u.id === userId);
    if (!target) return;

    if (!confirm(`Permanently delete user "${target.name}" (${target.email}) from workspace?`)) return;

    try {
      const res = await fetch(`/api/users/${userId}`, { method: 'DELETE' });
      const data = await res.json();
      if (data.success) {
        await this.loadAllData();
        this.render();
        alert(`User "${target.name}" deleted.`);
      } else {
        alert("Failed to delete user: " + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert("Error: " + e);
    }
  },

  /* ------------------------------------------------------------------------
     API Docs Tab
     ------------------------------------------------------------------------ */
  renderApiDocsTab() {
    return `
      <div>
        <div style="margin-bottom:20px;">
          <h2 style="font-size:20px; font-weight:700;">BijiOS REST API Specification</h2>
          <p style="font-size:13px; color:var(--text-muted); margin-top:4px;">
            OpenAPI 3.0 compatible endpoints for third-party integrations, external scripts, and automation pipelines.
          </p>
        </div>

        <div style="display:flex; flex-direction:column; gap:14px;">
          <!-- Endpoint: Auth -->
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px; border-left:4px solid #10b981;">
            <div style="display:flex; align-items:center; gap:10px; margin-bottom:8px;">
              <span style="background:#10b981; color:#fff; font-size:11px; font-weight:700; padding:2px 8px; border-radius:4px;">POST</span>
              <strong style="font-family:monospace; font-size:14px;">/api/auth/login</strong>
              <span style="font-size:12px; color:var(--text-muted); margin-left:auto;">User Authentication & JWT Session</span>
            </div>
            <div style="font-size:12px; background:rgba(0,0,0,0.04); padding:8px 10px; border-radius:6px; font-family:monospace;">
              curl -X POST "http://localhost:3000/api/auth/login" -H "Content-Type: application/json" -d '{"email":"admin@bijios.local","password":"password123"}'
            </div>
          </div>

          <!-- Endpoint: Users CRUD -->
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px; border-left:4px solid #f59e0b;">
            <div style="display:flex; align-items:center; gap:10px; margin-bottom:8px;">
              <span style="background:#f59e0b; color:#fff; font-size:11px; font-weight:700; padding:2px 8px; border-radius:4px;">POST / PUT / DELETE</span>
              <strong style="font-family:monospace; font-size:14px;">/api/users</strong>
              <span style="font-size:12px; color:var(--text-muted); margin-left:auto;">Workspace User RBAC Management</span>
            </div>
            <div style="font-size:12px; background:rgba(0,0,0,0.04); padding:8px 10px; border-radius:6px; font-family:monospace;">
              curl -X POST "http://localhost:3000/api/users" -H "Content-Type: application/json" -d '{"name":"Alice","email":"alice@acme.com","role":"Admin"}'
            </div>
          </div>

          <!-- Endpoint: Files -->
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px; border-left:4px solid #0284c7;">
            <div style="display:flex; align-items:center; gap:10px; margin-bottom:8px;">
              <span style="background:#0284c7; color:#fff; font-size:11px; font-weight:700; padding:2px 8px; border-radius:4px;">GET / POST / DELETE</span>
              <strong style="font-family:monospace; font-size:14px;">/api/files</strong>
              <span style="font-size:12px; color:var(--text-muted); margin-left:auto;">Files & Folders Tree Management</span>
            </div>
            <div style="font-size:12px; background:rgba(0,0,0,0.04); padding:8px 10px; border-radius:6px; font-family:monospace;">
              curl -H "X-User-Id: usr_dionipe_849" "http://localhost:3000/api/files?parent_id=all"
            </div>
          </div>

          <!-- Endpoint: Public Share -->
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px; border-left:4px solid #8b5cf6;">
            <div style="display:flex; align-items:center; gap:10px; margin-bottom:8px;">
              <span style="background:#8b5cf6; color:#fff; font-size:11px; font-weight:700; padding:2px 8px; border-radius:4px;">GET</span>
              <strong style="font-family:monospace; font-size:14px;">/api/shares/public/{token}</strong>
              <span style="font-size:12px; color:var(--text-muted); margin-left:auto;">External Public Document Viewer</span>
            </div>
            <div style="font-size:12px; background:rgba(0,0,0,0.04); padding:8px 10px; border-radius:6px; font-family:monospace;">
              curl "http://localhost:3000/api/shares/public/doku-pub-share-token"
            </div>
          </div>
        </div>
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     API Keys Tab
     ------------------------------------------------------------------------ */
  renderApiKeysTab() {
    return `
      <div>
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:18px;">
          <div>
            <h2 style="font-size:20px; font-weight:700;">API Integration Keys</h2>
            <p style="font-size:13px; color:var(--text-muted); margin-top:2px;">Generate and manage secret tokens for automation scripts and webhooks.</p>
          </div>
          <button class="mac-btn primary" onclick="DokuSettings.createApiKey()">+ Generate New Key</button>
        </div>

        <div style="display:flex; flex-direction:column; gap:10px;">
          ${this.apiKeys.map(k => `
            <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px; display:flex; justify-content:space-between; align-items:center;">
              <div>
                <div style="font-weight:600; font-size:14px;">${k.name}</div>
                <div style="font-family:monospace; font-size:12px; color:var(--accent-color); margin-top:4px;">${k.key}</div>
              </div>
              <button class="mac-btn" onclick="navigator.clipboard.writeText('${k.key}'); alert('API Key copied to clipboard!');">📋 Copy</button>
            </div>
          `).join('')}
        </div>
      </div>
    `;
  },

  createApiKey() {
    const name = prompt("Key Label / Application Name:", "CI/CD Pipeline Key");
    if (!name) return;

    fetch('/api/settings/api-keys', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name })
    }).then(res => res.json()).then(data => {
      if (data.success) {
        alert("New API Key generated successfully!");
        this.loadAllData().then(() => this.render());
      }
    });
  },

  /* ------------------------------------------------------------------------
     TOON v4 Inspector Tab
     ------------------------------------------------------------------------ */
  renderToonTab() {
    return `
      <div>
        <div style="margin-bottom:20px;">
          <h2 style="font-size:20px; font-weight:700;">TOON Format v4 Database Engine</h2>
          <p style="font-size:13px; color:var(--text-muted); margin-top:4px;">
            Conforms strictly to <a href="https://toonformat.dev" target="_blank" style="color:var(--accent-color);">toonformat.dev v4</a>: 
            Tabular token-oriented object notation delivering 37% - 55% LLM prompt token savings.
          </p>
        </div>

        <!-- Token Savings Summary Card -->
        <div style="background:linear-gradient(135deg, rgba(0,122,255,0.1), rgba(16,185,129,0.1)); border:1px solid rgba(0,122,255,0.25); border-radius:12px; padding:18px; margin-bottom:24px; display:flex; justify-content:space-around; text-align:center;">
          <div>
            <div style="font-size:12px; color:var(--text-muted);">Database Format</div>
            <div style="font-size:22px; font-weight:800; color:#007aff;">TOON v4 Tabular</div>
          </div>
          <div>
            <div style="font-size:12px; color:var(--text-muted);">Average Token Reduction</div>
            <div style="font-size:22px; font-weight:800; color:#10b981;">37.5% - 55%</div>
          </div>
          <div>
            <div style="font-size:12px; color:var(--text-muted);">Physical Storage</div>
            <div style="font-size:22px; font-weight:800;">data/db/*.toon</div>
          </div>
        </div>

        <!-- Tables Inspection -->
        <div style="display:flex; flex-direction:column; gap:16px;">
          ${this.toonStats.map(stat => `
            <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px;">
              <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px;">
                <strong style="font-size:14px; color:var(--text-main);">Table: ${stat.table}.toon</strong>
                <span style="font-size:12px; color:#10b981; font-weight:600;">
                  ⚡ Saves ~${stat.savings_percentage}% Tokens (${stat.toon_tokens} tokens vs ~${stat.json_estimated_tokens} JSON)
                </span>
              </div>
              <pre style="background:#1e1e24; color:#38bdf8; padding:12px; border-radius:8px; font-size:11px; overflow-x:auto; font-family:monospace; line-height:1.4;">${stat.raw_toon}</pre>
            </div>
          `).join('')}
        </div>
      </div>
    `;
  },

  /* ------------------------------------------------------------------------
     User Profile & Password Handlers
     ------------------------------------------------------------------------ */
  async saveProfileBasicInfo() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const name = document.getElementById('profile-name-input').value.trim();
    const email = document.getElementById('profile-email-input').value.trim();
    const cur = DokuHub.currentUser;
    if (!name || !email) {
      alert(isId ? "Nama dan email wajib diisi." : "Name and email are required.");
      return;
    }

    try {
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      const headers = { 'Content-Type': 'application/json' };
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const tid = (DokuHub.activeTenant && DokuHub.activeTenant.id) || cur.tenant_id || '';
      if (tid) headers['X-Tenant-Id'] = tid;
      headers['X-User-Id'] = cur.id;

      const res = await fetch(`/api/users/${cur.id}`, {
        method: 'PUT',
        headers,
        body: JSON.stringify({
          name,
          email,
          avatar: cur.avatar,
          color: cur.color
        })
      });
      const data = await res.json();
      if (res.ok && data.success) {
        cur.name = name;
        cur.email = email;
        if (data.user && data.user.avatar) {
          cur.avatar = data.user.avatar;
          localStorage.setItem(`dokuhub_custom_avatar_${cur.id}`, data.user.avatar);
        }
        if (data.user && data.user.color) {
          cur.color = data.user.color;
          localStorage.setItem(`dokuhub_custom_color_${cur.id}`, data.user.color);
        }
        localStorage.setItem('dokuhub_current_user', JSON.stringify(cur));
        sessionStorage.setItem('dokuhub_current_user', JSON.stringify(cur));

        const topName = document.getElementById('topbar-user-name');
        if (topName) topName.textContent = name;
        const dropName = document.getElementById('dropdown-user-name');
        const dropEmail = document.getElementById('dropdown-user-email');
        if (dropName) dropName.textContent = name;
        if (dropEmail) dropEmail.textContent = email;

        await this.loadAllData();
        this.render();
        alert(isId ? "✅ Profil pengguna berhasil diperbarui!" : "✅ User profile updated successfully!");
      } else {
        alert(data.error || (isId ? "Gagal memperbarui profil pengguna." : "Failed to update user profile."));
      }
    } catch (e) {
      alert(isId ? "Terjadi kesalahan jaringan." : "Network connection error.");
    }
  },

  async submitChangePassword() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const cur = DokuHub.currentUser;
    const old_password = document.getElementById('profile-old-pass').value;
    const new_password = document.getElementById('profile-new-pass').value;
    const confirm_pass = document.getElementById('profile-confirm-pass').value;
    const feedback = document.getElementById('password-change-feedback');

    if (!new_password) {
      if (feedback) feedback.innerHTML = `<span style="color:#ef4444;">⚠️ ${isId ? 'Kata sandi baru tidak boleh kosong.' : 'New password cannot be empty.'}</span>`;
      return;
    }
    if (new_password.length < 4) {
      if (feedback) feedback.innerHTML = `<span style="color:#ef4444;">⚠️ ${isId ? 'Kata sandi baru minimal 4 karakter.' : 'Password must be at least 4 characters.'}</span>`;
      return;
    }
    if (new_password !== confirm_pass) {
      if (feedback) feedback.innerHTML = `<span style="color:#ef4444;">⚠️ ${isId ? 'Konfirmasi kata sandi tidak cocok.' : 'Password confirmation does not match.'}</span>`;
      return;
    }

    try {
      const res = await fetch(`/api/users/${cur.id}/password`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'X-Tenant-Id': DokuHub.activeTenant.id,
          'X-User-Id': cur.id
        },
        body: JSON.stringify({ old_password, new_password })
      });
      const data = await res.json();
      if (res.ok && data.success) {
        if (feedback) feedback.innerHTML = `<span style="color:#10b981; font-weight:700;">✅ ${isId ? 'Kata sandi berhasil diperbarui!' : 'Password updated successfully!'}</span>`;
        document.getElementById('profile-old-pass').value = '';
        document.getElementById('profile-new-pass').value = '';
        document.getElementById('profile-confirm-pass').value = '';
      } else {
        if (feedback) feedback.innerHTML = `<span style="color:#ef4444;">❌ ${data.error || (isId ? 'Gagal mengubah kata sandi' : 'Failed to update password')}</span>`;
      }
    } catch (e) {
      if (feedback) feedback.innerHTML = '<span style="color:#ef4444;">❌ Kesalahan koneksi ke server.</span>';
    }
  },

  handleAvatarUpload(event) {
    const file = event.target.files && event.target.files[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = async (e) => {
      const dataUrl = e.target.result;
      const cur = DokuHub.currentUser;
      cur.avatar = dataUrl;

      // 1. Immediately persist to localStorage & sessionStorage
      localStorage.setItem(`dokuhub_custom_avatar_${cur.id}`, dataUrl);
      localStorage.setItem('dokuhub_current_user', JSON.stringify(cur));
      sessionStorage.setItem('dokuhub_current_user', JSON.stringify(cur));

      // 2. Immediately update UI
      const preview = document.getElementById('profile-avatar-preview');
      if (preview) {
        preview.innerHTML = `<img src="${dataUrl}" style="width:100%; height:100%; border-radius:50%; object-fit:cover;">`;
        preview.style.background = 'transparent';
      }

      const topAvatar = document.getElementById('topbar-user-avatar');
      if (topAvatar) {
        topAvatar.innerHTML = `<img src="${dataUrl}" style="width:100%; height:100%; border-radius:50%; object-fit:cover;">`;
        topAvatar.style.background = 'transparent';
      }

      // 3. Persist to backend database via REST API
      try {
        const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
        const headers = { 'Content-Type': 'application/json' };
        if (token) headers['Authorization'] = `Bearer ${token}`;
        const tid = (DokuHub.activeTenant && DokuHub.activeTenant.id) || cur.tenant_id || '';
        if (tid) headers['X-Tenant-Id'] = tid;
        headers['X-User-Id'] = cur.id;

        const res = await fetch(`/api/users/${cur.id}`, {
          method: 'PUT',
          headers,
          body: JSON.stringify({
            avatar: dataUrl,
            color: cur.color
          })
        });
        if (res.ok) {
          const resp = await res.json();
          if (resp.user && resp.user.avatar) {
            cur.avatar = resp.user.avatar;
            localStorage.setItem(`dokuhub_custom_avatar_${cur.id}`, resp.user.avatar);
            localStorage.setItem('dokuhub_current_user', JSON.stringify(cur));
            sessionStorage.setItem('dokuhub_current_user', JSON.stringify(cur));
          }
        }
      } catch (err) {
        console.warn("Could not save avatar to backend:", err);
      }
    };
    reader.readAsDataURL(file);
  },

  async selectAvatarPreset(preset) {
    const cur = DokuHub.currentUser;
    cur.avatar = preset;

    // 1. Immediately persist to client storage
    localStorage.setItem(`dokuhub_custom_avatar_${cur.id}`, preset);
    localStorage.setItem('dokuhub_current_user', JSON.stringify(cur));
    sessionStorage.setItem('dokuhub_current_user', JSON.stringify(cur));

    // 2. Immediately update UI
    const preview = document.getElementById('profile-avatar-preview');
    if (preview) {
      preview.textContent = preset;
      preview.style.background = cur.color || '#007aff';
    }

    const topAvatar = document.getElementById('topbar-user-avatar');
    if (topAvatar) {
      topAvatar.textContent = preset;
      topAvatar.style.background = cur.color || '#007aff';
    }

    // 3. Persist to backend database
    try {
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      const headers = { 'Content-Type': 'application/json' };
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const tid = (DokuHub.activeTenant && DokuHub.activeTenant.id) || cur.tenant_id || '';
      if (tid) headers['X-Tenant-Id'] = tid;
      headers['X-User-Id'] = cur.id;

      await fetch(`/api/users/${cur.id}`, {
        method: 'PUT',
        headers,
        body: JSON.stringify({
          avatar: preset,
          color: cur.color
        })
      });
    } catch (e) {
      console.warn("Could not save preset to backend:", e);
    }
  },

  async setAvatarColor(color) {
    const cur = DokuHub.currentUser;
    cur.color = color;

    // 1. Immediately persist to client storage
    localStorage.setItem(`dokuhub_custom_color_${cur.id}`, color);
    localStorage.setItem('dokuhub_current_user', JSON.stringify(cur));
    sessionStorage.setItem('dokuhub_current_user', JSON.stringify(cur));

    // 2. Immediately update UI
    const isImg = cur.avatar && (cur.avatar.startsWith('data:image') || cur.avatar.startsWith('/api/users/') || cur.avatar.startsWith('http'));
    const preview = document.getElementById('profile-avatar-preview');
    if (preview && !isImg) {
      preview.style.background = color;
    }

    const topAvatar = document.getElementById('topbar-user-avatar');
    if (topAvatar && !isImg) {
      topAvatar.style.background = color;
    }

    // 3. Persist to backend database
    try {
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      const headers = { 'Content-Type': 'application/json' };
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const tid = (DokuHub.activeTenant && DokuHub.activeTenant.id) || cur.tenant_id || '';
      if (tid) headers['X-Tenant-Id'] = tid;
      headers['X-User-Id'] = cur.id;

      await fetch(`/api/users/${cur.id}`, {
        method: 'PUT',
        headers,
        body: JSON.stringify({ color })
      });
    } catch (e) {}

    this.render();
  },

  async resetAvatarDefault() {
    const cur = DokuHub.currentUser;
    const initials = (cur.name || 'User').split(' ').map(n=>n[0]).join('').slice(0, 2).toUpperCase();
    cur.avatar = initials;
    cur.color = '#007aff';

    // 1. Clear custom avatar override
    localStorage.removeItem(`dokuhub_custom_avatar_${cur.id}`);
    localStorage.removeItem(`dokuhub_custom_color_${cur.id}`);
    localStorage.setItem('dokuhub_current_user', JSON.stringify(cur));
    sessionStorage.setItem('dokuhub_current_user', JSON.stringify(cur));

    // 2. Immediately update UI
    const preview = document.getElementById('profile-avatar-preview');
    if (preview) {
      preview.textContent = initials;
      preview.style.background = '#007aff';
    }

    const topAvatar = document.getElementById('topbar-user-avatar');
    if (topAvatar) {
      topAvatar.textContent = initials;
      topAvatar.style.background = '#007aff';
    }

    // 3. Persist reset to backend database
    try {
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      const headers = { 'Content-Type': 'application/json' };
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const tid = (DokuHub.activeTenant && DokuHub.activeTenant.id) || cur.tenant_id || '';
      if (tid) headers['X-Tenant-Id'] = tid;
      headers['X-User-Id'] = cur.id;

      await fetch(`/api/users/${cur.id}`, {
        method: 'PUT',
        headers,
        body: JSON.stringify({
          avatar: initials,
          color: '#007aff'
        })
      });
    } catch (e) {}

    this.render();
  },

  /* ------------------------------------------------------------------------
     Docking, Theme & Appearance Handlers
     ------------------------------------------------------------------------ */
  applyDockSize(size) {
    size = parseInt(size, 10) || 52;
    document.documentElement.style.setProperty('--dock-item-size', `${size}px`);
    localStorage.setItem('dokuhub_dock_size', size);
    const label = document.getElementById('dock-size-label');
    if (label) label.textContent = `${size} px`;
    const slider = document.getElementById('dock-size-slider');
    if (slider) slider.value = size;
  },

  toggleDockMagnify(enabled) {
    document.documentElement.style.setProperty('--dock-hover-scale', enabled ? '1.18' : '1.0');
    localStorage.setItem('dokuhub_dock_magnify', enabled);
    this.render();
  },

  applyDockPosition(pos) {
    const dockContainer = document.querySelector('.mac-dock-container');
    if (dockContainer) {
      dockContainer.classList.remove('dock-left', 'dock-right');
      if (pos === 'left') dockContainer.classList.add('dock-left');
      if (pos === 'right') dockContainer.classList.add('dock-right');
    }
    localStorage.setItem('dokuhub_dock_position', pos);
    this.render();
  },

  applyThemeMode(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('dokuhub_theme', theme);
    this.render();
  },

  applyAccentColor(color) {
    document.documentElement.style.setProperty('--accent-color', color);
    localStorage.setItem('dokuhub_accent_color', color);
    this.render();
  },

  /* ------------------------------------------------------------------------
     Security & Audit Trail Tab
     ------------------------------------------------------------------------ */
  auditLogs: [],
  auditStats: { total: 0, today: 0, warning: 0, critical: 0 },
  auditFilterCat: 'ALL',
  auditFilterSev: 'ALL',
  auditSearch: '',

  async loadAuditLogs() {
    try {
      const headers = {};
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const curUser = (window.DokuHub && DokuHub.currentUser) || null;
      if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

      const params = new URLSearchParams();
      if (this.auditFilterCat && this.auditFilterCat !== 'ALL') params.set('category', this.auditFilterCat);
      if (this.auditFilterSev && this.auditFilterSev !== 'ALL') params.set('severity', this.auditFilterSev);
      if (this.auditSearch && this.auditSearch.trim()) params.set('search', this.auditSearch.trim());
      params.set('limit', '250');

      const res = await fetch(`/api/audit/logs?${params.toString()}`, { headers });
      if (res.ok) {
        const data = await res.json();
        this.auditStats = data.stats || { total: 0, today: 0, warning: 0, critical: 0 };
        this.auditLogs = data.logs || [];
        if (this.activeTab === 'audit') {
          const pane = document.getElementById('settings-pane-content');
          if (pane) pane.innerHTML = this.renderAuditTab();
        }
      }
    } catch (e) {
      console.warn('Failed to load audit logs:', e);
    }
  },

  renderAuditTab() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const cur = (window.DokuHub && DokuHub.currentUser) || { role: 'Member' };
    const isSuperAdmin = cur.role === 'SuperAdmin';
    const st = this.auditStats || { total: 0, today: 0, warning: 0, critical: 0 };
    const locale = isId ? 'id-ID' : 'en-US';

    const sevBadge = (sev) => {
      if (sev === 'critical') return `<span style="background:rgba(239,68,68,0.16); color:#ef4444; padding:2px 7px; border-radius:6px; font-size:10px; font-weight:700;">CRITICAL</span>`;
      if (sev === 'warning') return `<span style="background:rgba(245,158,11,0.16); color:#d97706; padding:2px 7px; border-radius:6px; font-size:10px; font-weight:700;">WARNING</span>`;
      return `<span style="background:rgba(16,185,129,0.15); color:#10b981; padding:2px 7px; border-radius:6px; font-size:10px; font-weight:700;">INFO</span>`;
    };

    return `
      <div>
        <div style="display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:12px; margin-bottom:18px;">
          <div>
            <h2 style="font-size:20px; font-weight:800; margin:0 0 4px;">🛡️ ${isId ? 'Audit Trail & Pelacakan Aktivitas' : 'Security Audit Trail & Activity Log'}</h2>
            <p style="font-size:12.5px; color:var(--text-muted); margin:0;">
              ${isId ? 'Rekaman permanen seluruh aktivitas pengguna, autentikasi, manipulasi berkas, berbagi tautan, dan perubahan sistem.' : 'Append-only security trail of user logins, file operations, share links, and administrative actions.'}
            </p>
          </div>
          <div style="display:flex; gap:8px;">
            <button class="mac-btn" onclick="DokuSettings.loadAuditLogs()">🔄 ${isId ? 'Muat Ulang' : 'Refresh'}</button>
            <button class="mac-btn primary" onclick="DokuSettings.exportAuditCsv()">⬇️ ${isId ? 'Ekspor CSV' : 'Export CSV'}</button>
            ${isSuperAdmin ? `<button class="mac-btn" style="color:#ef4444;" onclick="DokuSettings.clearAuditLogs()">🗑️ ${isId ? 'Bersihkan' : 'Clear'}</button>` : ''}
          </div>
        </div>

        <!-- 4 Security KPI Cards -->
        <div style="display:grid; grid-template-columns:repeat(4, 1fr); gap:10px; margin-bottom:16px;">
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:12px 14px;">
            <div style="font-size:11px; color:var(--text-muted); font-weight:600;">${isId ? 'Total Aktivitas' : 'Total Events'}</div>
            <div style="font-size:22px; font-weight:800; color:var(--text-main); margin-top:2px;">${st.total}</div>
          </div>
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:12px 14px;">
            <div style="font-size:11px; color:var(--text-muted); font-weight:600;">${isId ? '24 Jam Terakhir' : 'Last 24 Hours'}</div>
            <div style="font-size:22px; font-weight:800; color:var(--accent-color); margin-top:2px;">${st.today}</div>
          </div>
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:12px 14px;">
            <div style="font-size:11px; color:var(--text-muted); font-weight:600;">${isId ? 'Peringatan (Warning)' : 'Warnings'}</div>
            <div style="font-size:22px; font-weight:800; color:#d97706; margin-top:2px;">${st.warning}</div>
          </div>
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:12px 14px;">
            <div style="font-size:11px; color:var(--text-muted); font-weight:600;">${isId ? 'Aksi Kritis (Critical)' : 'Critical Actions'}</div>
            <div style="font-size:22px; font-weight:800; color:#ef4444; margin-top:2px;">${st.critical}</div>
          </div>
        </div>

        <!-- Filter & Search Bar -->
        <div style="display:flex; gap:8px; flex-wrap:wrap; margin-bottom:14px;">
          <input type="text" value="${this.auditSearch || ''}"
                 placeholder="${isId ? '🔍 Cari user, aksi, detail, IP...' : '🔍 Search user, action, detail, IP...'}"
                 oninput="DokuSettings.auditSearch=this.value; clearTimeout(DokuSettings._auditSearchT); DokuSettings._auditSearchT=setTimeout(()=>DokuSettings.loadAuditLogs(), 280);"
                 style="flex:1; min-width:200px; padding:7px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:12.5px; outline:none;">

          <select onchange="DokuSettings.auditFilterCat=this.value; DokuSettings.loadAuditLogs();"
                  style="padding:7px 10px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:12px;">
            ${['ALL', 'AUTH', 'FILES', 'SHARES', 'NOTES', 'MEET', 'GDRIVE', 'ADMIN', 'DESKTOP'].map(c =>
              `<option value="${c}" ${this.auditFilterCat === c ? 'selected' : ''}>${c === 'ALL' ? (isId ? 'Semua Kategori' : 'All Categories') : c}</option>`
            ).join('')}
          </select>

          <select onchange="DokuSettings.auditFilterSev=this.value; DokuSettings.loadAuditLogs();"
                  style="padding:7px 10px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:12px;">
            <option value="ALL" ${this.auditFilterSev === 'ALL' ? 'selected' : ''}>${isId ? 'Semua Tingkat' : 'All Severities'}</option>
            <option value="info" ${this.auditFilterSev === 'info' ? 'selected' : ''}>INFO</option>
            <option value="warning" ${this.auditFilterSev === 'warning' ? 'selected' : ''}>WARNING</option>
            <option value="critical" ${this.auditFilterSev === 'critical' ? 'selected' : ''}>CRITICAL</option>
          </select>
        </div>

        <!-- Audit Log Table -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; overflow:hidden;">
          <div style="overflow-x:auto; max-height:380px; overflow-y:auto;">
            <table style="width:100%; border-collapse:collapse; font-size:11.5px;">
              <thead>
                <tr style="background:var(--window-header-bg); border-bottom:1px solid var(--window-border); text-align:left; color:var(--text-muted);">
                  <th style="padding:9px 10px;">${isId ? 'Waktu' : 'Timestamp'}</th>
                  <th style="padding:9px 10px;">${isId ? 'Pengguna' : 'User'}</th>
                  <th style="padding:9px 10px;">${isId ? 'Kategori & Aksi' : 'Category & Action'}</th>
                  <th style="padding:9px 10px;">${isId ? 'Rincian Aktivitas' : 'Activity Detail'}</th>
                  <th style="padding:9px 10px;">${isId ? 'Risiko' : 'Severity'}</th>
                  <th style="padding:9px 10px;">IP</th>
                </tr>
              </thead>
              <tbody>
                ${this.auditLogs.length === 0 ? `
                  <tr>
                    <td colspan="6" style="padding:28px; text-align:center; color:var(--text-muted);">
                      ${isId ? 'Belum ada catatan aktivitas yang cocok dengan filter.' : 'No audit log entries match your filter.'}
                    </td>
                  </tr>
                ` : this.auditLogs.map(log => {
                  const dt = log.timestamp ? new Date(log.timestamp * 1000).toLocaleString(locale, { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit', second: '2-digit' }) : '-';
                  return `
                    <tr style="border-bottom:1px solid var(--window-border);">
                      <td style="padding:8px 10px; white-space:nowrap; font-family:monospace; color:var(--text-muted);">${dt}</td>
                      <td style="padding:8px 10px; white-space:nowrap;">
                        <div style="font-weight:700; color:var(--text-main);">${log.user_name || log.user_id}</div>
                        <div style="font-size:10px; color:var(--text-muted);">${log.role || '-'}</div>
                      </td>
                      <td style="padding:8px 10px; white-space:nowrap;">
                        <span style="font-size:10px; font-weight:700; padding:2px 6px; border-radius:4px; background:rgba(0,122,255,0.12); color:var(--accent-color);">${log.category}</span>
                        <div style="font-size:10.5px; font-weight:600; margin-top:2px;">${log.action}</div>
                      </td>
                      <td style="padding:8px 10px; max-width:260px; word-break:break-word;">
                        <div>${log.detail || '-'}</div>
                        <div style="font-size:10px; color:var(--text-muted); font-family:monospace;">${log.method} ${log.resource} (${log.status_code})</div>
                      </td>
                      <td style="padding:8px 10px; white-space:nowrap;">${sevBadge(log.severity)}</td>
                      <td style="padding:8px 10px; white-space:nowrap; font-family:monospace; color:var(--text-muted);">${log.ip_address || '-'}</td>
                    </tr>
                  `;
                }).join('')}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    `;
  },

  async exportAuditCsv() {
    try {
      const headers = {};
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const curUser = (window.DokuHub && DokuHub.currentUser) || null;
      if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

      const res = await fetch('/api/audit/export', { headers });
      if (res.ok) {
        const csv = await res.text();
        const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = `dokuhub_audit_trail_${new Date().toISOString().slice(0, 10)}.csv`;
        a.click();
        URL.revokeObjectURL(a.href);
      }
    } catch (e) {
      alert('Export error: ' + e);
    }
  },

  async clearAuditLogs() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (!confirm(isId ? 'Bersihkan seluruh riwayat audit trail? Tindakan ini akan dicatat sebagai aksi kritis.' : 'Clear all audit trail logs? This action itself will be recorded as critical.')) return;
    try {
      const headers = {};
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const curUser = (window.DokuHub && DokuHub.currentUser) || null;
      if (curUser && curUser.tenant_id) headers['X-Tenant-Id'] = curUser.tenant_id;

      const res = await fetch('/api/audit/logs', { method: 'DELETE', headers });
      if (res.ok) {
        await this.loadAuditLogs();
        if (window.DokuIsland) {
          DokuIsland.info(isId ? 'Riwayat audit trail telah dibersihkan' : 'Audit trail cleared', 'Security');
        }
      }
    } catch (e) {
      alert('Error: ' + e);
    }
  },

  /* ------------------------------------------------------------------------
     Document Share Approval Flow & Comments
     ------------------------------------------------------------------------ */
  async loadShareApprovals() {
    try {
      const headers = {};
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      if (DokuHub.activeTenant && DokuHub.activeTenant.id) headers['X-Tenant-Id'] = DokuHub.activeTenant.id;
      if (DokuHub.currentUser && DokuHub.currentUser.id) headers['X-User-Id'] = DokuHub.currentUser.id;

      const res = await fetch('/api/shares', { headers });
      if (res.ok) {
        const data = await res.json();
        this.shareApprovals = data.shares || [];
        this.render();
      }
    } catch (e) {
      console.error('Failed to load share approvals:', e);
    }
  },

  renderShareApprovalsTab() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const cur = DokuHub.currentUser || { role: 'Staff' };
    const isApprover = ['SuperAdmin', 'Admin', 'Supervisor'].includes(cur.role);
    const list = this.shareApprovals || [];

    const pendingCount = list.filter(s => s.approval_status === 'pending').length;
    const approvedCount = list.filter(s => s.approval_status === 'approved').length;
    const rejectedCount = list.filter(s => s.approval_status === 'rejected').length;

    const filter = this.shareApprovalFilter || 'all';
    const filtered = list.filter(s => {
      if (filter === 'all') return true;
      return s.approval_status === filter;
    });

    const permBadge = (p) => {
      if (p === 'edit') return `<span style="padding:2px 8px; border-radius:10px; font-size:11px; font-weight:700; background:rgba(16,185,129,0.16); color:#059669;">✏️ Editor</span>`;
      if (p === 'comment') return `<span style="padding:2px 8px; border-radius:10px; font-size:11px; font-weight:700; background:rgba(245,158,11,0.16); color:#d97706;">💬 Comment</span>`;
      return `<span style="padding:2px 8px; border-radius:10px; font-size:11px; font-weight:700; background:rgba(59,130,246,0.16); color:#2563eb;">👁️ View Only</span>`;
    };

    const statusBadge = (st) => {
      if (st === 'pending') return `<span style="padding:2px 8px; border-radius:10px; font-size:11px; font-weight:700; background:rgba(245,158,11,0.18); color:#d97706;">⏳ Menunggu Supervisor</span>`;
      if (st === 'rejected') return `<span style="padding:2px 8px; border-radius:10px; font-size:11px; font-weight:700; background:rgba(239,68,68,0.16); color:#dc2626;">❌ Ditolak</span>`;
      return `<span style="padding:2px 8px; border-radius:10px; font-size:11px; font-weight:700; background:rgba(16,185,129,0.16); color:#059669;">✅ Disetujui</span>`;
    };

    const typeBadge = (t) => {
      if (t === 'public') return `<span style="font-size:11px; color:#0284c7; font-weight:600;">🌐 Publik (Auto)</span>`;
      if (t === 'external') return `<span style="font-size:11px; color:#7c3aed; font-weight:600;">✉️ External</span>`;
      return `<span style="font-size:11px; color:#059669; font-weight:600;">👥 Internal Staff</span>`;
    };

    return `
      <div style="max-width:980px;">
        <div style="display:flex; justify-content:space-between; align-items:flex-start; margin-bottom:18px; flex-wrap:wrap; gap:10px;">
          <div>
            <h2 style="font-size:20px; font-weight:700;">✅ ${isId ? 'Persetujuan Share Dokumen & Catatan' : 'Document Share Approvals & Comments'}</h2>
            <p style="font-size:12px; color:var(--text-muted); margin-top:4px;">
              ${isApprover
                ? (isId ? 'Anda memiliki kewenangan Approval (Supervisor / Admin). Tinjau pengajuan share Internal/External dari Staff untuk menjaga kerahasiaan dokumen.' : 'You have Approval authority (Supervisor / Admin). Review Internal/External share requests from Staff.')
                : (isId ? 'Pantau status pengajuan share dokumen Anda ke Supervisor, serta akses dokumen yang dibagikan kepada Anda.' : 'Track your document share requests to your Supervisor and access documents shared with you.')}
            </p>
          </div>
          <button class="mac-btn primary" onclick="DokuSettings.loadShareApprovals()">🔄 Refresh</button>
        </div>

        <!-- Summary Cards -->
        <div style="display:grid; grid-template-columns:repeat(3, 1fr); gap:12px; margin-bottom:18px;">
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px; cursor:pointer; ${filter === 'pending' ? 'border-color:#f59e0b;' : ''}" onclick="DokuSettings.shareApprovalFilter='pending'; DokuSettings.render();">
            <div style="font-size:11px; color:var(--text-muted); font-weight:600;">⏳ MENUNGGU APPROVAL</div>
            <div style="font-size:24px; font-weight:800; color:#d97706; margin-top:4px;">${pendingCount}</div>
          </div>
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px; cursor:pointer; ${filter === 'approved' ? 'border-color:#10b981;' : ''}" onclick="DokuSettings.shareApprovalFilter='approved'; DokuSettings.render();">
            <div style="font-size:11px; color:var(--text-muted); font-weight:600;">✅ DISETUJUI / AKTIF</div>
            <div style="font-size:24px; font-weight:800; color:#059669; margin-top:4px;">${approvedCount}</div>
          </div>
          <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:10px; padding:14px; cursor:pointer; ${filter === 'rejected' ? 'border-color:#ef4444;' : ''}" onclick="DokuSettings.shareApprovalFilter='rejected'; DokuSettings.render();">
            <div style="font-size:11px; color:var(--text-muted); font-weight:600;">❌ DITOLAK</div>
            <div style="font-size:24px; font-weight:800; color:#dc2626; margin-top:4px;">${rejectedCount}</div>
          </div>
        </div>

        <!-- Filter Pills -->
        <div style="display:flex; gap:8px; margin-bottom:14px; flex-wrap:wrap;">
          ${['all', 'pending', 'approved', 'rejected'].map(f => `
            <button class="mac-btn ${filter === f ? 'primary' : ''}" style="font-size:12px;" onclick="DokuSettings.shareApprovalFilter='${f}'; DokuSettings.render();">
              ${f === 'all' ? 'Semua (' + list.length + ')' : f === 'pending' ? '⏳ Pending (' + pendingCount + ')' : f === 'approved' ? '✅ Approved (' + approvedCount + ')' : '❌ Rejected (' + rejectedCount + ')'}
            </button>
          `).join('')}
        </div>

        <!-- Shares Table -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; overflow:hidden;">
          <table style="width:100%; border-collapse:collapse; font-size:12px;">
            <thead>
              <tr style="border-bottom:1px solid var(--window-border); text-align:left; color:var(--text-muted); font-size:11px; background:rgba(128,128,128,0.06);">
                <th style="padding:10px 12px;">DOKUMEN & TIPE</th>
                <th style="padding:10px 12px;">PENGAJU → APPROVER</th>
                <th style="padding:10px 12px;">IZIN AKSES</th>
                <th style="padding:10px 12px;">STATUS</th>
                <th style="padding:10px 12px; text-align:right;">AKSI</th>
              </tr>
            </thead>
            <tbody>
              ${filtered.length === 0 ? `
                <tr>
                  <td colspan="5" style="padding:28px; text-align:center; color:var(--text-muted);">
                    Belum ada data share dokumen pada filter ini. Ajukan share dari aplikasi <strong>Finder</strong> (klik ikon 🔗 pada dokumen).
                  </td>
                </tr>
              ` : filtered.map(s => `
                <tr style="border-bottom:1px solid var(--window-border);">
                  <td style="padding:10px 12px;">
                    <div style="font-weight:700; font-size:13px;">📄 ${s.file_name || s.file_id}</div>
                    <div style="margin-top:3px; display:flex; align-items:center; gap:8px; flex-wrap:wrap;">
                      ${typeBadge(s.share_type)}
                      ${s.target_email ? `<span style="font-size:11px; color:var(--text-muted);">→ ${s.target_email}</span>` : ''}
                    </div>
                    ${s.request_note ? `<div style="font-size:11px; color:var(--text-muted); margin-top:4px; font-style:italic;">"${s.request_note}"</div>` : ''}
                  </td>
                  <td style="padding:10px 12px;">
                    <div style="font-weight:600;">👤 ${s.requester_name || s.owner_id}</div>
                    ${s.requester_department ? `<div style="font-size:10px; color:var(--text-muted);">🏷️ ${s.requester_department}</div>` : ''}
                    ${s.approver_name ? `<div style="font-size:11px; color:var(--text-muted); margin-top:3px;">↳ Spv: <strong>${s.approver_name}</strong></div>` : ''}
                  </td>
                  <td style="padding:10px 12px;">
                    ${permBadge(s.permission)}
                  </td>
                  <td style="padding:10px 12px;">
                    ${statusBadge(s.approval_status)}
                    ${s.approval_note ? `<div style="font-size:10px; color:var(--text-muted); margin-top:4px;">Catatan: ${s.approval_note}</div>` : ''}
                  </td>
                  <td style="padding:10px 12px; text-align:right; white-space:nowrap;">
                    <div style="display:inline-flex; gap:6px; flex-wrap:wrap; justify-content:flex-end;">
                      ${(s.approval_status === 'pending' && s.can_approve) ? `
                        <button class="mac-btn primary" style="font-size:11px; padding:3px 9px; background:#10b981; border-color:#10b981;" onclick="DokuSettings.reviewShare('${s.id}', true)">✓ Setujui</button>
                        <button class="mac-btn" style="font-size:11px; padding:3px 9px; color:#ef4444; border-color:rgba(239,68,68,0.4);" onclick="DokuSettings.reviewShare('${s.id}', false)">✗ Tolak</button>
                      ` : ''}
                      ${(s.approval_status === 'approved' && (s.permission === 'comment' || s.permission === 'edit')) ? `
                        <button class="mac-btn" style="font-size:11px; padding:3px 9px;" onclick="DokuSettings.openShareCommentsModal('${s.id}', '${(s.file_name || '').replace(/'/g, "\\'")}', '${s.permission}')">💬 Catatan</button>
                      ` : ''}
                      ${s.approval_status === 'approved' ? `
                        <button class="mac-btn" style="font-size:11px; padding:3px 9px;" onclick="DokuQR.copyText(window.location.origin + '/s/${s.token}').then(ok => { if(window.DokuIsland) DokuIsland.info(ok ? 'Link share disalin!' : 'Gagal menyalin otomatis', 'Share'); else alert(ok ? 'Link share disalin!' : 'Gagal menyalin'); });">🔗 Link</button>
                        <button class="mac-btn" style="font-size:11px; padding:3px 9px;" onclick="if(window.DokuQR) DokuQR.openModal({ title: 'QR Share & Download', fileName: '${(s.file_name || 'dokumen').replace(/'/g, "\\'")}', shareUrl: window.location.origin + '/s/${s.token}', downloadUrl: window.location.origin + '/api/files/${s.file_id}/download' });">📱 QR</button>
                      ` : ''}
                    </div>
                  </td>
                </tr>
              `).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `;
  },

  async reviewShare(shareId, approve) {
    const note = prompt(
      approve
        ? 'Catatan persetujuan Supervisor (opsional):'
        : 'Alasan penolakan share dokumen (opsional):',
      approve ? 'Disetujui sesuai prosedur' : 'Dokumen bersifat rahasia'
    );
    if (note === null) return;

    try {
      const headers = { 'Content-Type': 'application/json' };
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      if (DokuHub.activeTenant && DokuHub.activeTenant.id) headers['X-Tenant-Id'] = DokuHub.activeTenant.id;
      if (DokuHub.currentUser && DokuHub.currentUser.id) headers['X-User-Id'] = DokuHub.currentUser.id;

      const endpoint = approve ? `/api/shares/${shareId}/approve` : `/api/shares/${shareId}/reject`;
      const res = await fetch(endpoint, {
        method: 'POST',
        headers,
        body: JSON.stringify({ note })
      });
      const data = await res.json();
      if (data.success) {
        if (window.DokuIsland) {
          DokuIsland.info(
            approve ? 'Pengajuan share dokumen disetujui.' : 'Pengajuan share dokumen ditolak.',
            'Approval Flow'
          );
        }
        await this.loadShareApprovals();
      } else {
        alert('Gagal memproses approval: ' + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert('Error: ' + e);
    }
  },

  async openShareCommentsModal(shareId, fileName, permission) {
    let existing = document.getElementById('modal-share-comments');
    if (existing) existing.remove();

    const headers = {};
    const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
    if (token) headers['Authorization'] = `Bearer ${token}`;
    if (DokuHub.activeTenant && DokuHub.activeTenant.id) headers['X-Tenant-Id'] = DokuHub.activeTenant.id;
    if (DokuHub.currentUser && DokuHub.currentUser.id) headers['X-User-Id'] = DokuHub.currentUser.id;

    let comments = [];
    try {
      const res = await fetch(`/api/shares/${shareId}/comments`, { headers });
      if (res.ok) {
        const data = await res.json();
        comments = data.comments || [];
      }
    } catch (e) {}

    const overlay = document.createElement('div');
    overlay.id = 'modal-share-comments';
    overlay.style.cssText = 'position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(8px); z-index:999999; display:flex; align-items:center; justify-content:center;';
    overlay.innerHTML = `
      <div style="width:480px; max-height:80vh; display:flex; flex-direction:column; background:var(--window-bg); border:1px solid var(--window-border); border-radius:16px; padding:20px; box-shadow:0 30px 70px rgba(0,0,0,0.4);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:12px;">
          <div>
            <h3 style="font-size:15px; font-weight:700;">💬 Catatan Dokumen</h3>
            <div style="font-size:11px; color:var(--text-muted);">${fileName} • Izin: <strong>${permission}</strong></div>
          </div>
          <button class="traffic-btn btn-close" onclick="document.getElementById('modal-share-comments').remove()"></button>
        </div>

        <div id="share-comments-list" style="flex:1; overflow-y:auto; max-height:280px; display:flex; flex-direction:column; gap:8px; margin-bottom:12px; padding:8px; border-radius:10px; background:var(--card-bg); border:1px solid var(--window-border);">
          ${comments.length === 0 ? `<div style="font-size:12px; color:var(--text-muted); text-align:center; padding:16px;">Belum ada catatan pada dokumen ini.</div>` : comments.map(c => `
            <div style="padding:8px 10px; border-radius:8px; background:var(--window-bg); border:1px solid var(--window-border);">
              <div style="display:flex; justify-content:space-between; font-size:11px; margin-bottom:4px;">
                <strong>${c.author_name || 'User'}</strong>
                <span style="color:var(--text-muted);">${c.created_at || ''}</span>
              </div>
              <div style="font-size:12px;">${c.comment}</div>
            </div>
          `).join('')}
        </div>

        <div style="display:flex; gap:8px;">
          <input type="text" id="share-new-comment-input" placeholder="Tulis catatan / komentar..." style="flex:1; padding:8px 10px; border:1px solid var(--window-border); border-radius:8px; font-size:12px; background:var(--card-bg); color:var(--text-main);">
          <button class="mac-btn primary" onclick="DokuSettings.submitShareComment('${shareId}', '${fileName.replace(/'/g, "\\'")}', '${permission}')">Kirim</button>
        </div>
      </div>
    `;
    document.body.appendChild(overlay);
  },

  async submitShareComment(shareId, fileName, permission) {
    const input = document.getElementById('share-new-comment-input');
    const comment = input ? input.value.trim() : '';
    if (!comment) return;

    try {
      const headers = { 'Content-Type': 'application/json' };
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      if (DokuHub.activeTenant && DokuHub.activeTenant.id) headers['X-Tenant-Id'] = DokuHub.activeTenant.id;
      if (DokuHub.currentUser && DokuHub.currentUser.id) headers['X-User-Id'] = DokuHub.currentUser.id;

      const res = await fetch(`/api/shares/${shareId}/comments`, {
        method: 'POST',
        headers,
        body: JSON.stringify({ comment })
      });
      const data = await res.json();
      if (data.success) {
        this.openShareCommentsModal(shareId, fileName, permission);
      } else {
        alert('Gagal menambah catatan: ' + (data.error || 'Unknown error'));
      }
    } catch (e) {
      alert('Error: ' + e);
    }
  }
};