//! Argumentos `--chave valor` e `--bandeira` dos comandos do xtask (sem
//! clap, PLANO.md). Argumento desconhecido é erro, para um erro de digitação
//! não virar opção ignorada em silêncio.

#[derive(Debug, Default)]
pub struct Args {
    valores: Vec<(String, String)>,
    bandeiras: Vec<String>,
    posicionais: Vec<String>,
}

impl Args {
    /// `com_valor`: opções que levam valor; `bandeiras`: opções sem valor.
    /// Tudo que não começa com `--` é posicional.
    pub fn ler(args: &[String], com_valor: &[&str], bandeiras: &[&str]) -> Result<Args, String> {
        let mut saida = Args::default();
        let mut i = 0;
        while i < args.len() {
            let a = &args[i];
            if com_valor.contains(&a.as_str()) {
                let valor = args
                    .get(i + 1)
                    .ok_or_else(|| format!("falta o valor de {a}"))?;
                saida.valores.push((a.clone(), valor.clone()));
                i += 2;
            } else if bandeiras.contains(&a.as_str()) {
                saida.bandeiras.push(a.clone());
                i += 1;
            } else if a.starts_with("--") {
                return Err(format!("argumento desconhecido: {a}"));
            } else {
                saida.posicionais.push(a.clone());
                i += 1;
            }
        }
        Ok(saida)
    }

    /// Último valor dado para `nome`.
    pub fn valor(&self, nome: &str) -> Option<&str> {
        self.valores
            .iter()
            .rev()
            .find(|(n, _)| n == nome)
            .map(|(_, v)| v.as_str())
    }

    pub fn obrigatorio(&self, nome: &str) -> Result<&str, String> {
        self.valor(nome).ok_or_else(|| format!("falta {nome}"))
    }

    pub fn bandeira(&self, nome: &str) -> bool {
        self.bandeiras.iter().any(|b| b == nome)
    }

    pub fn posicionais(&self) -> &[String] {
        &self.posicionais
    }
}

/// `48x48` → (48, 48).
pub fn tamanho(texto: &str) -> Result<(u32, u32), String> {
    let (w, h) = texto
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("tamanho inválido: «{texto}» (use LxA, como 48x48)"))?;
    let n = |s: &str| {
        s.trim()
            .parse::<u32>()
            .ok()
            .filter(|n| (1..=1024).contains(n))
            .ok_or_else(|| format!("tamanho inválido: «{texto}»"))
    };
    Ok((n(w)?, n(h)?))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn v(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn valores_bandeiras_e_posicionais() {
        let a = Args::ler(
            &v(&[
                "dir",
                "--celula",
                "48x48",
                "--contorno",
                "--id",
                "a",
                "--id",
                "b",
            ]),
            &["--celula", "--id"],
            &["--contorno"],
        )
        .unwrap();
        assert_eq!(a.valor("--celula"), Some("48x48"));
        assert_eq!(a.valor("--id"), Some("b"), "vale o último");
        assert!(a.bandeira("--contorno"));
        assert!(!a.bandeira("--outra"));
        assert_eq!(a.posicionais(), &["dir".to_owned()]);
        assert!(a.obrigatorio("--saida").is_err());
    }

    #[test]
    fn desconhecido_e_valor_faltando_sao_erro() {
        assert!(Args::ler(&v(&["--nada"]), &[], &[]).is_err());
        assert!(Args::ler(&v(&["--id"]), &["--id"], &[]).is_err());
    }

    #[test]
    fn tamanhos() {
        assert_eq!(tamanho("48x48"), Ok((48, 48)));
        assert_eq!(tamanho("32X16"), Ok((32, 16)));
        assert!(tamanho("48").is_err());
        assert!(tamanho("0x4").is_err());
    }
}
