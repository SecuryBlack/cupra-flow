# Subir a GitHub

## 1. Crear repositorio en GitHub

Ve a https://github.com/new y crea un repositorio:
- **Nombre:** `cupraflow`
- **Descripción:** Agente de gestión de red y balanceo de carga
- **Público o Privado:** Tu elección
- **NO inicializar** con README, .gitignore ni license

## 2. Conectar repositorio local con GitHub

Ejecuta en la terminal (dentro de `D:\cupraflow`):

```bash
git remote add origin https://github.com/TU_USUARIO/cupraflow.git
```

## 3. Subir código

```bash
git push -u origin master
```

Si usas HTTPS, te pedirá tu token de GitHub.
Si usas SSH:
```bash
git remote add origin git@github.com:TU_USUARIO/cupraflow.git
```

## 4. Verificar

Ve a `https://github.com/TU_USUARIO/cupraflow` y deberías ver todos los archivos.

## Próximos pasos

Una vez en GitHub, podemos:
1. Crear GitHub Actions para builds automáticos
2. Configurar releases
3. Implementar el auto-updater (Fase 2)

## Nota

Si ya tienes un repositorio existente en `D:\cupraflow\.git`, está listo para usar.
Solo necesitas conectarlo con GitHub remoto.