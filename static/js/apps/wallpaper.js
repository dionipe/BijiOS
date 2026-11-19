/**
 * Wallpaper Chooser for DokuHub macOS Desktop
 * Supports Built-in Themes, Custom Image Uploads (.jpg, .png, .bmp, .webp),
 * and Files from Finder with per-user personalization!
 */

window.DokuWallpaper = {
  container: null,
  userImages: [],

  async init(containerEl) {
    this.container = containerEl;
    await this.fetchUserImages();
    this.render();
  },

  async fetchUserImages() {
    try {
      const headers = {};
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;
      const tenantId = (DokuHub.activeTenant && DokuHub.activeTenant.id) || (DokuHub.currentUser && DokuHub.currentUser.tenant_id) || '';
      if (tenantId) headers['X-Tenant-Id'] = tenantId;

      const res = await fetch('/api/files?parent_id=all&category=all', { headers });
      if (res.ok) {
        const files = await res.json();
        if (Array.isArray(files)) {
          const imgExts = ['jpg', 'jpeg', 'png', 'bmp', 'webp', 'gif', 'svg'];
          this.userImages = files.filter(f => !f.is_folder && imgExts.includes((f.extension || '').toLowerCase()));
        }
      }
    } catch (e) {
      console.warn("Could not load user images for wallpaper:", e);
      this.userImages = [];
    }
  },

  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curUser = DokuHub.currentUser || { name: 'Super Admin', role: 'Superadmin' };
    const currentWp = DokuHub.currentWallpaper || (DokuHub.wallpapers[0] ? DokuHub.wallpapers[0].url : '');

    this.container.innerHTML = `
      <div style="padding:24px; height:100%; overflow-y:auto; background:var(--window-bg); font-family:var(--font-family);">
        <!-- Header -->
        <div style="display:flex; align-items:center; justify-content:space-between; margin-bottom:18px;">
          <div>
            <h2 style="font-size:20px; font-weight:700; margin-bottom:4px;">${isId ? 'Ubah Wallpaper Desktop' : 'Change Desktop Wallpaper'}</h2>
            <p style="font-size:12.5px; color:var(--text-muted);">
              ${isId ? 'Sesuaikan wallpaper desktop untuk akun:' : 'Personalize desktop wallpaper for:'}
              <strong style="color:var(--text-main); font-weight:600;">👤 ${curUser.name}</strong> 
              <span style="font-size:11px; padding:1px 6px; border-radius:4px; background:rgba(0,122,255,0.15); color:var(--accent-color); margin-left:4px;">${curUser.role}</span>
            </p>
          </div>
          <div>
            <label class="mac-btn mac-btn-primary" style="cursor:pointer; display:inline-flex; align-items:center; gap:6px; font-size:12px; padding:6px 14px;">
              ⬆️ ${isId ? 'Unggah Gambar...' : 'Upload Image...'}
              <input type="file" id="wallpaper-upload-input" accept="image/jpeg,image/png,image/bmp,image/webp,image/gif,image/svg+xml" style="display:none;" onchange="DokuWallpaper.handleUpload(event)">
            </label>
          </div>
        </div>

        <!-- Section 1: User Images from Finder (.jpg, .png, .bmp) -->
        ${this.userImages.length > 0 ? `
          <div style="margin-bottom:26px;">
            <div style="display:flex; align-items:center; justify-content:space-between; margin-bottom:10px;">
              <h3 style="font-size:13.5px; font-weight:700; text-transform:uppercase; letter-spacing:0.4px; color:var(--text-muted);">
                🖼️ ${isId ? 'Gambar dari Finder & Berkas Saya (.jpg, .png, .bmp)' : 'Images from My Finder & Files (.jpg, .png, .bmp)'}
              </h3>
              <span style="font-size:11px; color:var(--text-muted);">${this.userImages.length} ${isId ? 'gambar tersedia' : 'images available'}</span>
            </div>
            <div style="display:grid; grid-template-columns:repeat(auto-fill, minmax(170px, 1fr)); gap:14px;">
              ${this.userImages.map(img => {
      const src = img.content && (img.content.startsWith('data:') || img.content.startsWith('http'))
        ? img.content
        : `/api/files/${img.id}/download`;
      const isSelected = (currentWp === src);
      return `
                  <div style="cursor:pointer; border-radius:10px; overflow:hidden; border:2px solid ${isSelected ? 'var(--accent-color)' : 'var(--window-border)'}; box-shadow:0 4px 12px rgba(0,0,0,0.08); transition:transform 0.15s; position:relative; background:var(--card-bg);"
                       onclick="DokuWallpaper.select('${src}')"
                       onmouseover="this.style.transform='scale(1.02)'"
                       onmouseout="this.style.transform='scale(1)'">
                    <div style="height:105px; background-image:url('${src}'); background-size:cover; background-position:center; background-color:rgba(125,125,125,0.1);"></div>
                    ${isSelected ? `<div style="position:absolute; top:6px; right:6px; background:var(--accent-color); color:#fff; border-radius:50%; width:20px; height:20px; display:flex; align-items:center; justify-content:center; font-size:11px; box-shadow:0 2px 6px rgba(0,0,0,0.3);">✓</div>` : ''}
                    <div style="padding:6px 10px; font-size:11px; font-weight:600; text-align:center; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                      ${img.name}
                    </div>
                  </div>
                `;
    }).join('')}
            </div>
          </div>
        ` : ''}

        <!-- Section 2: Built-in Wallpapers Grid -->
        <div style="margin-bottom:26px;">
          <h3 style="font-size:13.5px; font-weight:700; text-transform:uppercase; letter-spacing:0.4px; color:var(--text-muted); margin-bottom:10px;">
            🍏 ${isId ? 'Tema Bawaan macOS' : 'Built-in macOS Wallpaper Themes'}
          </h3>
          <div style="display:grid; grid-template-columns:repeat(auto-fill, minmax(170px, 1fr)); gap:14px;">
            ${DokuHub.wallpapers.map(wp => {
      const isSelected = (currentWp === wp.url);
      return `
                <div style="cursor:pointer; border-radius:10px; overflow:hidden; border:2px solid ${isSelected ? 'var(--accent-color)' : 'var(--window-border)'}; box-shadow:0 4px 12px rgba(0,0,0,0.08); transition:transform 0.15s; position:relative; background:var(--card-bg);" 
                     onclick="DokuWallpaper.select('${wp.url}')"
                     onmouseover="this.style.transform='scale(1.02)'" 
                     onmouseout="this.style.transform='scale(1)'">
                  <div style="height:105px; background-image:url('${wp.url}'); background-size:cover; background-position:center;"></div>
                  ${isSelected ? `<div style="position:absolute; top:6px; right:6px; background:var(--accent-color); color:#fff; border-radius:50%; width:20px; height:20px; display:flex; align-items:center; justify-content:center; font-size:11px; box-shadow:0 2px 6px rgba(0,0,0,0.3);">✓</div>` : ''}
                  <div style="padding:6px 10px; font-size:11px; font-weight:600; text-align:center;">
                    ${wp.name}
                  </div>
                </div>
              `;
    }).join('')}
          </div>
        </div>

        <!-- Section 3: Custom Image URL -->
        <div style="background:var(--card-bg); border:1px solid var(--window-border); border-radius:12px; padding:16px;">
          <h3 style="font-size:13px; font-weight:700; margin-bottom:8px;">${isId ? 'Gunakan Gambar via URL Khusus' : 'Custom Image URL'}</h3>
          <div style="display:flex; gap:8px;">
            <input type="text" id="custom-wallpaper-url" placeholder="https://images.unsplash.com/photo-..." style="flex:1; padding:8px 12px; border-radius:6px; border:1px solid var(--window-border); background:rgba(125,125,125,0.06); color:var(--text-main); font-size:12.5px; outline:none;">
            <button class="mac-btn primary" onclick="DokuWallpaper.setCustom()">${isId ? 'Terapkan' : 'Apply'}</button>
          </div>
        </div>
      </div>
    `;
  },

  select(url) {
    DokuHub.setWallpaper(url, true);
    this.render();
  },

  setCustom() {
    const input = document.getElementById('custom-wallpaper-url');
    if (input && input.value.trim()) {
      DokuHub.setWallpaper(input.value.trim(), true);
      this.render();
    }
  },

  async handleUpload(e) {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    const file = files[0];
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    const reader = new FileReader();
    reader.onload = async (event) => {
      const dataUrl = event.target.result;

      // 1. Immediately apply as this user's personalized wallpaper
      DokuHub.setWallpaper(dataUrl, true);

      // 2. Also save to user's Finder files so it persists in the workspace
      try {
        const currentUserId = DokuHub.currentUser ? DokuHub.currentUser.id : 'usr_dionipe_849';
        const currentTenantId = (DokuHub.activeTenant && DokuHub.activeTenant.id) || (DokuHub.currentUser ? DokuHub.currentUser.tenant_id : '');
        const ext = (file.name.split('.').pop() || 'jpg').toLowerCase();

        await fetch('/api/files', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Tenant-Id': currentTenantId,
            'X-User-Id': currentUserId
          },
          body: JSON.stringify({
            name: file.name,
            extension: ext,
            content: dataUrl,
            parent_id: 'root',
            owner_id: currentUserId,
            tenant_id: currentTenantId,
            size: file.size
          })
        });

        await this.fetchUserImages();
        this.render();

        if (window.DokuFinder && typeof DokuFinder.loadAllFiles === 'function') {
          DokuFinder.loadAllFiles();
        }

        alert(isId ? `✅ "${file.name}" berhasil diunggah dan dijadikan wallpaper desktop!` : `✅ "${file.name}" uploaded and set as desktop wallpaper!`);
      } catch (err) {
        console.warn("Could not save wallpaper to files:", err);
      }
    };

    reader.readAsDataURL(file);
    e.target.value = '';
  }
};
