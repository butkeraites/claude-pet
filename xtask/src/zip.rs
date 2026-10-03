//! Leitor mínimo de `.zip` em memória, para `cargo xtask zeca --pack
//! <arquivo.zip>` ler o pack sem extrair nada no disco.
//!
//! Só o necessário para os zips de packs de pixel art: o diretório central,
//! entradas guardadas (método 0) ou comprimidas com deflate (método 8), sem
//! criptografia e sem zip64. O CRC-32 de cada entrada é conferido e o
//! tamanho descompactado tem teto, contra zip-bomba.

use std::io::Read;

/// Maior entrada descompactada aceita.
pub const LIMITE_ENTRADA: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entrada {
    pub nome: String,
    metodo: u16,
    crc: u32,
    tamanho: u64,
    comprimido: u64,
    /// Onde começa o cabeçalho local.
    local: u64,
}

pub struct Zip<'a> {
    dados: &'a [u8],
    pub entradas: Vec<Entrada>,
}

fn u16_em(d: &[u8], i: usize) -> Result<u16, String> {
    d.get(i..i + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| "zip truncado".to_owned())
}

fn u32_em(d: &[u8], i: usize) -> Result<u32, String> {
    d.get(i..i + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| "zip truncado".to_owned())
}

impl<'a> Zip<'a> {
    pub fn ler(dados: &'a [u8]) -> Result<Zip<'a>, String> {
        // Fim do diretório central: assinatura 0x06054b50 nos últimos
        // 22 + 65535 bytes (o comentário do zip pode ter até 64 KiB).
        let inicio_busca = dados.len().saturating_sub(22 + 65535);
        let fim = (inicio_busca..dados.len().saturating_sub(21))
            .rev()
            .find(|&i| u32_em(dados, i) == Ok(0x0605_4b50))
            .ok_or("não é um zip (sem o fim do diretório central)")?;
        let total = u16_em(dados, fim + 10)? as usize;
        let mut pos = u32_em(dados, fim + 16)? as usize;
        let mut entradas = Vec::with_capacity(total);
        for _ in 0..total {
            if u32_em(dados, pos)? != 0x0201_4b50 {
                return Err("diretório central do zip corrompido".into());
            }
            let bandeiras = u16_em(dados, pos + 8)?;
            let metodo = u16_em(dados, pos + 10)?;
            let crc = u32_em(dados, pos + 16)?;
            let comprimido = u32_em(dados, pos + 20)? as u64;
            let tamanho = u32_em(dados, pos + 24)? as u64;
            let n_nome = u16_em(dados, pos + 28)? as usize;
            let n_extra = u16_em(dados, pos + 30)? as usize;
            let n_comentario = u16_em(dados, pos + 32)? as usize;
            let local = u32_em(dados, pos + 42)? as u64;
            let nome = dados
                .get(pos + 46..pos + 46 + n_nome)
                .ok_or("zip truncado")?;
            if bandeiras & 1 != 0 {
                return Err("zip com criptografia não é suportado".into());
            }
            if [comprimido, tamanho, local].contains(&0xFFFF_FFFF) {
                return Err("zip64 não é suportado".into());
            }
            entradas.push(Entrada {
                nome: String::from_utf8_lossy(nome).into_owned(),
                metodo,
                crc,
                tamanho,
                comprimido,
                local,
            });
            pos += 46 + n_nome + n_extra + n_comentario;
        }
        Ok(Zip { dados, entradas })
    }

    /// Conteúdo descompactado de uma entrada, com o CRC conferido.
    pub fn conteudo(&self, e: &Entrada) -> Result<Vec<u8>, String> {
        if e.tamanho > LIMITE_ENTRADA {
            return Err(format!("«{}» descompactado passa de 64 MiB", e.nome));
        }
        let l = e.local as usize;
        if u32_em(self.dados, l)? != 0x0403_4b50 {
            return Err(format!("cabeçalho local de «{}» corrompido", e.nome));
        }
        let inicio =
            l + 30 + u16_em(self.dados, l + 26)? as usize + u16_em(self.dados, l + 28)? as usize;
        let bruto = self
            .dados
            .get(inicio..inicio + e.comprimido as usize)
            .ok_or_else(|| format!("«{}» truncado", e.nome))?;
        let mut saida = Vec::with_capacity(e.tamanho as usize);
        match e.metodo {
            0 => saida.extend_from_slice(bruto),
            8 => {
                flate2::read::DeflateDecoder::new(bruto)
                    .take(LIMITE_ENTRADA + 1)
                    .read_to_end(&mut saida)
                    .map_err(|err| format!("«{}»: deflate inválido: {err}", e.nome))?;
            }
            m => return Err(format!("«{}» usa o método {m} (só 0 e 8)", e.nome)),
        }
        if saida.len() as u64 != e.tamanho {
            return Err(format!("«{}» com tamanho diferente do declarado", e.nome));
        }
        if crc32fast::hash(&saida) != e.crc {
            return Err(format!("«{}» com CRC errado (zip corrompido)", e.nome));
        }
        Ok(saida)
    }
}

#[cfg(test)]
pub(crate) mod testes {
    use std::io::Write;

    use super::*;

    /// Monta um zip com as entradas dadas (deflate se `comprimir`).
    pub(crate) fn zip_sintetico(entradas: &[(&str, &[u8])], comprimir: bool) -> Vec<u8> {
        let mut saida = Vec::new();
        let mut central = Vec::new();
        for (nome, conteudo) in entradas {
            let (metodo, dados) = if comprimir {
                let mut c =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
                c.write_all(conteudo).unwrap();
                (8u16, c.finish().unwrap())
            } else {
                (0u16, conteudo.to_vec())
            };
            let crc = crc32fast::hash(conteudo);
            let local = saida.len() as u32;
            let mut cab = Vec::new();
            cab.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            cab.extend_from_slice(&[20, 0, 0, 0]);
            cab.extend_from_slice(&metodo.to_le_bytes());
            cab.extend_from_slice(&[0; 4]);
            cab.extend_from_slice(&crc.to_le_bytes());
            cab.extend_from_slice(&(dados.len() as u32).to_le_bytes());
            cab.extend_from_slice(&(conteudo.len() as u32).to_le_bytes());
            cab.extend_from_slice(&(nome.len() as u16).to_le_bytes());
            cab.extend_from_slice(&[0, 0]);
            cab.extend_from_slice(nome.as_bytes());
            saida.extend_from_slice(&cab);
            saida.extend_from_slice(&dados);
            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
            central.extend_from_slice(&metodo.to_le_bytes());
            central.extend_from_slice(&[0; 4]);
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(dados.len() as u32).to_le_bytes());
            central.extend_from_slice(&(conteudo.len() as u32).to_le_bytes());
            central.extend_from_slice(&(nome.len() as u16).to_le_bytes());
            central.extend_from_slice(&[0; 2 + 2 + 2 + 2 + 4]);
            central.extend_from_slice(&local.to_le_bytes());
            central.extend_from_slice(nome.as_bytes());
        }
        let inicio_central = saida.len() as u32;
        saida.extend_from_slice(&central);
        saida.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        saida.extend_from_slice(&[0; 4]);
        saida.extend_from_slice(&(entradas.len() as u16).to_le_bytes());
        saida.extend_from_slice(&(entradas.len() as u16).to_le_bytes());
        saida.extend_from_slice(&(central.len() as u32).to_le_bytes());
        saida.extend_from_slice(&inicio_central.to_le_bytes());
        saida.extend_from_slice(&[0, 0]);
        saida
    }

    #[test]
    fn le_entradas_guardadas_e_comprimidas() {
        for comprimir in [false, true] {
            let z = zip_sintetico(&[("a/b.txt", b"ola ola ola ola"), ("c", b"")], comprimir);
            let zip = Zip::ler(&z).unwrap();
            let nomes: Vec<&str> = zip.entradas.iter().map(|e| e.nome.as_str()).collect();
            assert_eq!(nomes, ["a/b.txt", "c"]);
            assert_eq!(zip.conteudo(&zip.entradas[0]).unwrap(), b"ola ola ola ola");
            assert_eq!(zip.conteudo(&zip.entradas[1]).unwrap(), b"");
        }
    }

    #[test]
    fn crc_errado_e_lixo_sao_erro() {
        let mut z = zip_sintetico(&[("a", b"conteudo")], false);
        let i = z.windows(8).position(|w| w == b"conteudo").unwrap();
        z[i] = b'C';
        let zip = Zip::ler(&z).unwrap();
        assert!(zip.conteudo(&zip.entradas[0]).unwrap_err().contains("CRC"));
        assert!(Zip::ler(b"isto nao e um zip de verdade....").is_err());
    }
}
