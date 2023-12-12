# Instalación

## Opción 1: Descarga del Binario (Linux)

Descarga la última versión de Faucet para Linux desde la
[página de lanzamientos en GitHub](https://github.com/andyquinterom/faucet/releases).

```bash
FAUCET_VERSION="v0.3.1"

wget https://github.com/andyquinterom/Faucet/releases/download/$FAUCET_VERSION/faucet-x86_64-unknown-linux-musl -O faucet

# Haz el binario ejecutable
chmod +x faucet

# Mueve el binario a un directorio en tu PATH (por ejemplo, el binario local del usuario)
mv faucet ~/.local/bin
```

> **Nota:**
> Aunque se espera que la descarga del binario funcione en la mayoría de las distribuciones de Linux,
> no se garantiza la compatibilidad con todos los sistemas. Si encuentras problemas,
> considera usar la instalación con Cargo o las opciones de compilación desde el origen.

## Opción 2: Instalación con Cargo (Linux, macOS, Windows)

Instala Faucet con Cargo, el gestor de paquetes de Rust.

1. Instala Rust siguiendo las instrucciones [aquí](https://www.rust-lang.org/tools/install).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Instala Faucet con Cargo.

```bash
cargo install faucet-server --version 0.3.1
```

## Opción 3: Compilar desde el Código Fuente (Linux, macOS, Windows)

1. Instala Rust siguiendo las instrucciones [aquí](https://www.rust-lang.org/tools/install).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clona el repositorio de Faucet.

```bash
git clone https://github.com/andyquinterom/Faucet.git
```

3. Compila Faucet con Cargo.

```bash
cargo install --path .
```
