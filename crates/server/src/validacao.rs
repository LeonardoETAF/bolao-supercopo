use crate::errors::AppError;

/// Mantém apenas os dígitos de uma string (remove pontuação, espaços, etc.).
pub fn somente_digitos(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Valida e normaliza um e-mail (trim + lowercase).
/// Verificação básica de formato: `local@dominio.tld`, sem espaços.
pub fn validar_email(email: &str) -> Result<String, AppError> {
    let e = email.trim().to_lowercase();

    if e.is_empty() || e.contains(' ') || e.len() > 254 {
        return Err(AppError::EmailInvalido);
    }

    let partes: Vec<&str> = e.split('@').collect();
    if partes.len() != 2 {
        return Err(AppError::EmailInvalido);
    }
    let (local, dominio) = (partes[0], partes[1]);

    if local.is_empty() || dominio.is_empty() || !dominio.contains('.') {
        return Err(AppError::EmailInvalido);
    }

    // Cada rótulo do domínio deve ser não-vazio (rejeita "a..b", ".a", "a.").
    if dominio.split('.').any(|p| p.is_empty()) {
        return Err(AppError::EmailInvalido);
    }

    // Caracteres permitidos (conjunto pragmático).
    let permitido = |c: char| {
        c.is_ascii_alphanumeric() || "._%+-@".contains(c)
    };
    if !e.chars().all(permitido) {
        return Err(AppError::EmailInvalido);
    }

    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_email_valido() {
        assert_eq!(validar_email("  Leo@Email.com ").unwrap(), "leo@email.com");
        assert!(validar_email("a.b+c@dominio.com.br").is_ok());
    }

    #[test]
    fn rejeita_email_sem_arroba() {
        assert!(validar_email("leoemail.com").is_err());
    }

    #[test]
    fn rejeita_email_sem_dominio() {
        assert!(validar_email("leo@").is_err());
        assert!(validar_email("leo@semponto").is_err());
        assert!(validar_email("leo @email.com").is_err());
    }
}
