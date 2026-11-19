//! RBAC and Multitenant Authentication Module

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    SuperAdmin,
    Admin,
    Supervisor,
    Member,
    Guest,
}

impl Role {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "superadmin" | "super_admin" => Role::SuperAdmin,
            "admin" | "admin_tenant" => Role::Admin,
            "supervisor" | "spv" => Role::Supervisor,
            "member" | "staff" | "user" => Role::Member,
            _ => Role::Guest,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::SuperAdmin => "SuperAdmin",
            Role::Admin => "Admin",
            Role::Supervisor => "Supervisor",
            Role::Member => "Staff",
            Role::Guest => "Guest",
        }
    }

    pub fn can_manage_tenant(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::Admin)
    }

    pub fn can_manage_users(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::Admin)
    }

    pub fn can_approve_shares(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::Admin | Role::Supervisor)
    }

    pub fn can_write_files(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::Admin | Role::Supervisor | Role::Member)
    }

    pub fn can_create_public_shares(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::Admin | Role::Supervisor | Role::Member)
    }

    pub fn can_manage_system(&self) -> bool {
        matches!(self, Role::SuperAdmin)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub tenant_id: String,
    pub email: String,
    pub name: String,
    pub role: Role,
}

impl AuthContext {
    pub fn guest() -> Self {
        Self {
            user_id: "guest".to_string(),
            tenant_id: "public".to_string(),
            email: "guest@bijios.local".to_string(),
            name: "Guest User".to_string(),
            role: Role::Guest,
        }
    }
}
