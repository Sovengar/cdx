# Old PowerShell-based cdx (archived)
# Replaced by Rust TUI binary cdx-rs.
# Kept for historical reference.
#
# Original location: ~\Documents\PowerShell\Cdx.ps1
#
# ============================
# cdx ÔÇö CD Interactivo Unificado
# Jump:    cdx <name>       ÔåÆ cd directo ÔåÆ zoxide ÔåÆ TUI con query
# Browse:  cdx              ÔåÆ TUI fd (carpetas)
# Search:  cdx -g <query>   ÔåÆ ripgrep b├║squeda de contenido
# Help:    cdx -h            ÔåÆ muestra ayuda
# TUI:     fd/rg toggle Ctrl+G | dotfiles Ctrl+A | WinHidden Ctrl+W | home Ctrl+H
# ============================

$script:ExcludeDirs       = @('node_modules', '.git', '.cache', 'cache', 'licenses', 'vendor', 'target', 'build', 'dist', 'Modules', 'modules', 'lib', 'platform')
$script:ExcludeWinDirs    = @('AppData', 'ProgramData')
$script:ExcludePathGlobs  = @('**/go/pkg/mod')

function cdx {
    [CmdletBinding()]
    param(
        [Parameter(Position=0, ValueFromRemainingArguments=$true)]
        [string[]]$QueryParts,
        [Alias('g')]
        [switch]$Grep,
        [Alias('h')]
        [switch]$Help
    )

    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $Query = $QueryParts -join ' '

    # 1) -g ÔåÆ b├║squeda de contenido con ripgrep
    if ($Grep) {
        Invoke-CdxSearch -Query $Query
        return
    }

    # 2) -h / --help ÔåÆ mostrar ayuda
    if ($Help -or ($Query -in @('--help', '-help', '/?'))) {
        Show-CdxHelp
        return
    }

    # 3) Sin args ÔåÆ TUI de navegaci├│n
    if (-not $Query) {
        Invoke-CdxTui
        return
    }

    # 3) Shortcuts especiales ÔåÆ $HOME
    if ($Query -in @('~', '...')) {
        Set-Location $env:USERPROFILE
        ShowResult
        return
    }

    # 4) Con query ÔåÆ cd directo ÔåÆ zoxide ÔåÆ TUI con query pre-llenada
    if (Test-Path $Query) {
        Set-Location $Query
        ShowResult
        return
    }

    $hasZoxide = Get-Command zoxide -ErrorAction SilentlyContinue
    if ($hasZoxide) {
        $result = zoxide query $Query 2>$null
        if ($result) {
            Set-Location $result
            ShowResult
            return
        }
    }

    # Fallback: TUI con query pre-llenada
    Invoke-CdxTui -InitialQuery $Query
}

# ============================
# ShowResult ÔÇö ls + git hint tras cd
# ============================
function ShowResult {
    $path = (Get-Location).Path
    $display = if ($path.StartsWith($env:USERPROFILE)) {
        "~" + $path.Substring($env:USERPROFILE.Length).Replace('\', '/')
    } else {
        $path.Replace('\', '/')
    }
    Write-Host "`n$display" -ForegroundColor Cyan
    if (Get-Command eza -ErrorAction SilentlyContinue) {
        eza --icons --group-directories-first
    } else {
        Get-ChildItem -Force | Format-Table
    }
    if (Get-Command git -ErrorAction SilentlyContinue) {
        $gitRoot = git rev-parse --show-toplevel 2>$null
        if ($gitRoot) {
            Write-Host "  Consider using: yazi, broot, nvim, lazygit, code ." -ForegroundColor DarkGray
        }
    }
}

# ============================
# Show-CdxHelp ÔÇö Pantalla de ayuda
# ============================
function Show-CdxHelp {
    $homeDir = if ($env:USERPROFILE) { $env:USERPROFILE } else { '~' }

@"
ÔòöÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòù
Ôòæ         cdx ÔÇö CD Interactivo Unificado   Ôòæ
ÔòÜÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòÉÔòØ

USO:
  cdx                    TUI ÔÇö navegar carpetas (fd)
  cdx <ruta>             cd directo si la ruta existe
  cdx <nombre>           zoxide ÔåÆ fallback a TUI
  cdx -g <query>         b├║squeda global por contenido
  cdx -h                 esta ayuda
  cdx ~                  ir a $homeDir
  cdx ...                ir a $homeDir

ATAJOS TUI:
  Enter       cd al directorio / abrir archivo
  Esc         subir al padre (cd ..)
  Ctrl+C      salir (se queda donde est├í)
  Ctrl+G      alternar fd (carpetas) Ôåö rg (archivos)
  Ctrl+A      alternar dotfiles (.*)
  Ctrl+W      alternar WinHidden (AppData, ProgramData)
  Ctrl+H      ir a $homeDir

DEPENDENCIAS:
  fd, fzf     obligatorias
  rg          obligatoria (TUI modo Search + -g)
  zoxide      opcional (merge de directorios frecuentes)
  eza, bat    opcionales (preview mejorado)
"@
}

# ============================
# Invoke-CdxTui ÔÇö TUI fd/rg
# ============================
function Invoke-CdxTui {
    param([string]$InitialQuery = '')

    # Check fd
    if (-not (Get-Command fd -ErrorAction SilentlyContinue)) {
        Write-Host '[!] fd not found. Install: winget install sharkdp.fd' -ForegroundColor Red
        return
    }

    # 1) Cache zoxide list ONCE
    $script:zoxideCache = @()
    $hasZoxide = Get-Command zoxide -ErrorAction SilentlyContinue
    if ($hasZoxide) {
        $script:zoxideCache = zoxide query --list 2>$null
    }
    Write-Host "[cdx] zoxide cache: $($script:zoxideCache.Count) entries" -ForegroundColor DarkGray

    # 2) State init
    $hasEza = Get-Command eza -ErrorAction SilentlyContinue
    $hasBat = Get-Command bat -ErrorAction SilentlyContinue
    $stateFile = Join-Path $env:TEMP 'cdx_state.txt'
    Set-Content -Path $stateFile -Value '2' -Force -NoNewline
    Write-Host "[cdx] state init: 2 (dotfiles=ON, WinHidden=OFF)" -ForegroundColor DarkGray

    function Format-DisplayPath {
        param([string]$Path)
        if ($Path.StartsWith($env:USERPROFILE)) {
            return "~" + $Path.Substring($env:USERPROFILE.Length).Replace('\', '/')
        }
        return $Path.Replace('\', '/')
    }

    # Reload script ÔÇö toggles state bit and regenerates items in real-time
    $reloadScript = Join-Path $env:TEMP 'cdx_reload.ps1'

    # Build inline arrays from script-scope variables (single source of truth)
    $excludeDirsInline = "'" + (($script:ExcludeDirs | ForEach-Object { $_ }) -join "','") + "'"
    $excludeWinDirsInline = "'" + (($script:ExcludeWinDirs | ForEach-Object { $_ }) -join "','") + "'"
    $excludePathGlobsInline = "'" + (($script:ExcludePathGlobs | ForEach-Object { $_ }) -join "','") + "'"

    @"
param([int]`$ToggleBit = 1)

`$sFile = "`$env:TEMP\cdx_state.txt"
`$oldS = [int](Get-Content `$sFile -Raw).Trim()
`$s = `$oldS -bxor `$ToggleBit
Set-Content -Path `$sFile -Value `$s -Force -NoNewline

`$rgMode       = (`$s -band 1) -ne 0
`$showDotfiles = (`$s -band 2) -ne 0
`$showWinHidden = (`$s -band 4) -ne 0
`$currentPath   = `$env:CDX_CURRENT_PATH

[Console]::Error.WriteLine("[cdx] toggle bit=`$(`$ToggleBit): `$oldS -> `$s (rg=`$rgMode, dot=`$showDotfiles, win=`$showWinHidden)")

# Injected exclude lists (single source of truth)
`$excludeDirs = @($excludeDirsInline)
`$excludeWinDirs = @($excludeWinDirsInline)
`$excludePathGlobs = @($excludePathGlobsInline)

# Mode line (consumed by --header-lines 1)
`$homePath      = `$env:USERPROFILE
`$displayPath = if (`$currentPath.StartsWith(`$homePath)) {
    "~" + `$currentPath.Substring(`$homePath.Length).Replace('\', '/')
} else { `$currentPath.Replace('\', '/') }
`$modeLabel = if (`$rgMode) { 'Grep mode' } else { 'Find mode' }
`$dotLabel = if (`$showDotfiles) { 'dotfiles: Ô£ô' } else { 'dotfiles: Ô£ù' }
`$winLabel = if (`$showWinHidden) { 'WinHidden: Ô£ô' } else { 'WinHidden: Ô£ù' }
"`$displayPath | `$modeLabel | `$dotLabel | `$winLabel"

if (`$rgMode) {
    `$cmdArgs = @('--files', `$currentPath, '--smart-case')
    if (`$showDotfiles -or `$showWinHidden) { `$cmdArgs += '--hidden' }
    foreach (`$d in `$excludeDirs) { `$cmdArgs += '--glob'; `$cmdArgs += "!`$d" }
    foreach (`$d in `$excludePathGlobs) { `$cmdArgs += '--glob'; `$cmdArgs += "!`$d" }
    if (-not `$showDotfiles) { `$cmdArgs += '--glob'; `$cmdArgs += '!.*' }
    if (-not `$showWinHidden) { foreach (`$d in `$excludeWinDirs) { `$cmdArgs += '--glob'; `$cmdArgs += "!`$d" } }
    [Console]::Error.WriteLine("[cdx] rg args: `$(`$cmdArgs -join ' ')")
    & rg @cmdArgs 2>`$null | ForEach-Object { `$_.Replace('\', '/') }
} else {
    `$isRoot = (`$currentPath -eq [System.IO.Path]::GetPathRoot(`$currentPath))
    if (`$isRoot) {
        if (`$showDotfiles -or `$showWinHidden) {
            if (-not `$showWinHidden) {
                Get-ChildItem -Directory -Path `$currentPath -Force -ErrorAction SilentlyContinue -Exclude `$excludeWinDirs | Select-Object -ExpandProperty Name
            } else {
                Get-ChildItem -Directory -Path `$currentPath -Force -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name
            }
        } else {
            Get-ChildItem -Directory -Path `$currentPath -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name
        }
    } else {
        `$cmdArgs = @('--base-directory', `$currentPath, '--type', 'd')
        if (`$showDotfiles -or `$showWinHidden) { `$cmdArgs += '--hidden' }
        foreach (`$d in `$excludeDirs) { `$cmdArgs += '--exclude'; `$cmdArgs += `$d }
        foreach (`$d in `$excludePathGlobs) { `$cmdArgs += '--exclude'; `$cmdArgs += `$d }
        if (-not `$showDotfiles) { `$cmdArgs += '--exclude'; `$cmdArgs += '.*' }
        if (-not `$showWinHidden) { foreach (`$d in `$excludeWinDirs) { `$cmdArgs += '--exclude'; `$cmdArgs += `$d } }
        `$cmdArgs += '.'
        [Console]::Error.WriteLine("[cdx] fd args: `$(`$cmdArgs -join ' ')")
        `$fdDirs = & fd @cmdArgs 2>`$null | ForEach-Object { `$_.Replace('\', '/').TrimEnd('/') }
        [Console]::Error.WriteLine("[cdx] fd raw: `$(`$fdDirs.Count) dirs")

        # Zoxide merge (only filter $ExcludeDirs)
        `$zoxPath = "`$env:TEMP\cdx_zoxide_cache.txt"
        `$zoxideCacheRel = Get-Content `$zoxPath 2>`$null
        `$zMap = @{}
        `$zDirs = @()
        foreach (`$z in `$zoxideCacheRel) {
            if (`$z -eq `$currentPath) { continue }
            `$prefix = `$currentPath.TrimEnd('\') + '\'
            if (`$z.StartsWith(`$prefix)) {
                `$rel = `$z.Substring(`$currentPath.Length).TrimStart('\').Replace('\', '/').TrimEnd('/')
                if (`$rel) {
                    `$parts = `$rel -split '/'
                    `$skip = `$false
                    foreach (`$p in `$parts) {
                        if (`$p -in `$excludeDirs) { `$skip = `$true; break }
                    }
                    if (-not `$skip -and `$rel -match '(^|/)go/pkg/mod($|/)') { `$skip = `$true }
                    if (-not `$skip -and -not `$zMap.ContainsKey(`$rel)) {
                        `$zMap[`$rel] = `$true
                        `$zDirs += `$rel
                    }
                }
            }
        }
        [Console]::Error.WriteLine("[cdx] zoxide merge: `$(`$zDirs.Count) starred")
        foreach (`$z in `$zDirs) { "Ôÿà `$z" }
        foreach (`$d in `$fdDirs) {
            if (-not `$zMap.ContainsKey(`$d)) { `$d }
        }
    }
}
"@ | Set-Content -Path $reloadScript -Force

    while ($true) {
        $currentPath = (Get-Location).Path
        $displayPath = Format-DisplayPath -Path $currentPath
        $state = [int]((Get-Content $stateFile -Raw).Trim())
        $rgMode        = ($state -band 1) -ne 0
        $showDotfiles  = ($state -band 2) -ne 0
        $showWinHidden = ($state -band 4) -ne 0
        $env:CDX_CURRENT_PATH = $currentPath
        $env:CDX_PREVIEW_BASE = $currentPath

        # Mode & state labels for header
        $enterLabel = if ($rgMode) { 'open' } else { 'cd' }
        $modeLabel  = if ($rgMode) { 'Grep mode' } else { 'Find mode' }
        $dotLabel   = if ($showDotfiles)  { 'dotfiles: Ô£ô' } else { 'dotfiles: Ô£ù' }
        $winLabel   = if ($showWinHidden) { 'WinHidden: Ô£ô' } else { 'WinHidden: Ô£ù' }

        Write-Host "[cdx] loop: $displayPath | state=$state | $modeLabel | $dotLabel | $winLabel" -ForegroundColor DarkGray

        # Header line 3 (state) ÔÇö computed BEFORE items, consumed by --header-lines 1
        $header3 = "$displayPath | $modeLabel | $dotLabel | $winLabel"

        # Brief loading indicator (cleared by fzf's alternate screen buffer)
        Write-Host "ÔÅ│" -NoNewline

        if ($rgMode) {
            # ===== MODO RG: listar ARCHIVOS =====
            $rgArgs = @('--files', '--smart-case')
            if ($showDotfiles -or $showWinHidden) { $rgArgs += '--hidden' }
            foreach ($d in $script:ExcludeDirs) { $rgArgs += '--glob'; $rgArgs += "!$d" }
            foreach ($p in $script:ExcludePathGlobs) { $rgArgs += '--glob'; $rgArgs += "!$p" }
            if (-not $showDotfiles) { $rgArgs += '--glob', '!.*' }
            if (-not $showWinHidden) { foreach ($c in $script:ExcludeWinDirs) { $rgArgs += '--glob'; $rgArgs += "!$c" } }
            $rgArgs += $currentPath

            Write-Host "[cdx] rg: $($rgArgs.Count) args, dir=$currentPath" -ForegroundColor DarkGray
            $rgOut = & rg @rgArgs 2>$null | ForEach-Object { $_.Replace('\', '/') }
            Write-Host "[cdx] rg results: $($rgOut.Count) files" -ForegroundColor DarkGray

            $items = @($header3) + $rgOut

            $preview = if ($hasBat) {
                "bat --color=always --line-range :50 --highlight-line {q} {}"
            } else {
                "Get-Content {} -TotalCount 50"
            }
        } else {
            # ===== MODO FD: listar CARPETAS =====
            $isRoot = ($currentPath -eq [System.IO.Path]::GetPathRoot($currentPath))

            if ($isRoot) {
                if ($showDotfiles -or $showWinHidden) {
                    if (-not $showWinHidden) {
                        $fdDirs = Get-ChildItem -Directory -Path $currentPath -Force -ErrorAction SilentlyContinue -Exclude 'AppData','ProgramData' | Select-Object -ExpandProperty Name
                    } else {
                        $fdDirs = Get-ChildItem -Directory -Path $currentPath -Force -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name
                    }
                } else {
                    $fdDirs = Get-ChildItem -Directory -Path $currentPath -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name
                }
                Write-Host "[cdx] root ls: $($fdDirs.Count) dirs" -ForegroundColor DarkGray
                $zoxideDirs = @()
                $zoxideMap = @{}
            } else {
                $fdArgs = @('--base-directory', $currentPath, '--type', 'd')
                if ($showDotfiles -or $showWinHidden) { $fdArgs += '--hidden' }
                foreach ($d in $script:ExcludeDirs) { $fdArgs += '--exclude'; $fdArgs += $d }
                foreach ($p in $script:ExcludePathGlobs) { $fdArgs += '--exclude'; $fdArgs += $p }
                if (-not $showDotfiles) { $fdArgs += '--exclude', '.*' }
                if (-not $showWinHidden) { foreach ($c in $script:ExcludeWinDirs) { $fdArgs += '--exclude'; $fdArgs += $c } }
                $fdArgs += '.'

                Write-Host "[cdx] fd: $($fdArgs.Count) args" -ForegroundColor DarkGray
                $fdDirs = & fd @fdArgs 2>$null | ForEach-Object { $_.Replace('\', '/').TrimEnd('/') }
                Write-Host "[cdx] fd results: $($fdDirs.Count) dirs" -ForegroundColor DarkGray

                # Zoxide merge (only filter $ExcludeDirs)
                $zoxideMap = @{}
                $zoxideDirs = @()
                foreach ($z in $script:zoxideCache) {
                    if ($z -eq $currentPath) { continue }
                    $prefix = $currentPath.TrimEnd('\') + '\'
                    if ($z.StartsWith($prefix)) {
                        $rel = $z.Substring($currentPath.Length).TrimStart('\').Replace('\', '/').TrimEnd('/')
                        if ($rel) {
                            $parts = $rel -split '/'
                            $skip = $false
                            foreach ($p in $parts) {
                                if ($p -in $script:ExcludeDirs) { $skip = $true; break }
                            }
                            if (-not $skip -and $rel -match '(^|/)go/pkg/mod($|/)') { $skip = $true }
                            if (-not $skip -and -not $zoxideMap.ContainsKey($rel)) {
                                $zoxideMap[$rel] = $true
                                $zoxideDirs += $rel
                            }
                        }
                    }
                }
                Write-Host "[cdx] zoxide merge: $($zoxideDirs.Count) starred" -ForegroundColor DarkGray
            }

            $items = @($header3)
            foreach ($z in $zoxideDirs) { $items += "Ôÿà $z" }
            foreach ($d in $fdDirs) {
                if (-not $zoxideMap.ContainsKey($d)) { $items += $d }
            }

            # Preview script para dirs (dir + git status)
            $previewScript = Join-Path $env:TEMP 'cdx_preview.ps1'
            $scriptContent = @'
param([string]$Path)
function Get-CdxDirectoryTree {
    param([string]$Path, [int]$Depth = 2)
    $exclude = @('node_modules', '.git', '.cache', 'cache', 'licenses', 'vendor', 'target', 'build', 'dist', 'Modules', 'modules', 'lib', 'platform', 'AppData', 'ProgramData')
    try {
        $items = Get-ChildItem -Path $Path -Force -ErrorAction SilentlyContinue | Where-Object { $exclude -notcontains $_.Name }
        foreach ($item in $items) {
            Write-Output $item.Name
            if ($item.PSIsContainer -and $Depth -ge 1) {
                $sub = Get-ChildItem -Path $item.FullName -Force -ErrorAction SilentlyContinue | Where-Object { $exclude -notcontains $_.Name }
                foreach ($s in $sub) {
                    Write-Output "  $($s.Name)"
                    if ($s.PSIsContainer -and $Depth -ge 2) {
                        $sub2 = Get-ChildItem -Path $s.FullName -Force -ErrorAction SilentlyContinue | Where-Object { $exclude -notcontains $_.Name }
                        foreach ($s2 in $sub2) {
                            Write-Output "    $($s2.Name)"
                        }
                    }
                }
            }
        }
    } catch {}
}

$Path = $Path.Replace('"','').Replace("'","")
$Path = $Path -replace '^Ôÿà ', ''
$basePath = $env:CDX_PREVIEW_BASE
if (-not $basePath) { $basePath = Get-Location }
$fullPath = Join-Path $basePath $Path

if (Test-Path $fullPath -PathType Container) {
    if (Get-Command eza -ErrorAction SilentlyContinue) {
        $dirOut = (eza --icons --group-directories-first --color=always $fullPath 2>$null) -join "`n"
    } else {
        $dirOut = (Get-ChildItem $fullPath | Format-Table Name,Mode,LastWriteTime | Out-String) -join "`n"
    }

    $treeOut = Get-CdxDirectoryTree -Path $fullPath -Depth 2 | Out-String

    $gitStatus = ''
    $gitDir = Join-Path $fullPath '.git'
    if (Test-Path $gitDir -PathType Container) {
        Push-Location $fullPath
        try {
            $gitStatus = git status --short 2>$null
            if (-not $gitStatus) { $gitStatus = 'Clean' }
        } catch {
            $gitStatus = 'Not a git repo'
        }
        Pop-Location
    }

    Write-Output "=== CONTENTS ==="
    Write-Output $dirOut
    Write-Output ""
    Write-Output "=== DIRECTORY TREE ==="
    Write-Output $treeOut
    Write-Output ""
    Write-Output "=== GIT STATUS ==="
    Write-Output $gitStatus
} elseif (Test-Path $fullPath) {
    $gitStatus = ''
    $gitDir = Split-Path $fullPath -Parent
    Push-Location $gitDir
    try {
        $gitRoot = git rev-parse --show-toplevel 2>$null
        if ($gitRoot) {
            $relativePath = ($fullPath -replace [regex]::Escape($gitRoot), '').Trim('\', '/')
            $gitStatus = git status --short $relativePath 2>$null
        }
    } catch {}
    Pop-Location

    $filePreview = ''
    if (Get-Command bat -ErrorAction SilentlyContinue) {
        $filePreview = bat --color=always --line-range :50 $fullPath 2>$null
    } else {
        $filePreview = Get-Content $fullPath -TotalCount 50
    }

    Write-Output $filePreview
    Write-Output ""
    Write-Output "=== GIT STATUS ==="
    Write-Output $gitStatus
}
'@
            Set-Content -Path $previewScript -Value $scriptContent -Force
            $preview = 'pwsh -NoProfile -File "{0}" {{}}' -f $previewScript
        }

        if ($items.Count -le 1) {
            Write-Host "[cdx] empty result, showing inline ls" -ForegroundColor DarkGray
            ShowResult
            return
        }

        # Header (above fzf prompt)
        $header = "Enter ($enterLabel) | Esc (cd ..) | Ctrl+C (Exit) | Ctrl+H (cd ~)`nCtrl+G (Grep) | Ctrl+A (dotfiles)`nCtrl+W (WinHidden) | Ctrl+O (yazi)"

        $env:FZF_DEFAULT_OPTS = '--height=80% --layout=reverse --border'

        # Build fzf args array
        $fzfArgs = @(
            "--header=$header",
            '--header-lines=1',
            "--preview=$preview",
            '--preview-window=right:35%,border-rounded',
            "--bind=ctrl-g:reload(pwsh -NoProfile -File `"$reloadScript`" -ToggleBit 1)",
            "--bind=ctrl-a:reload(pwsh -NoProfile -File `"$reloadScript`" -ToggleBit 2)",
            "--bind=ctrl-w:reload(pwsh -NoProfile -File `"$reloadScript`" -ToggleBit 4)",
            '--bind=ctrl-c:become(echo __EXIT_CDX__)',
            '--bind=ctrl-h:become(echo __GOTO_HOME__)',
            '--bind=ctrl-o:become(echo __CD_YAZI__{})'
        )
        if ($InitialQuery) {
            $fzfArgs += "--query=$InitialQuery"
            $InitialQuery = ''
        }

        Write-Host "[cdx] fzf: $($items.Count) items" -ForegroundColor DarkGray

        # Clear loading indicator before fzf
        Write-Host "`r  " -NoNewline

        # Run fzf
        try {
            $selected = @($items) | fzf @fzfArgs 2>$null
        } finally {
            $env:FZF_DEFAULT_OPTS = ''
        }

        # Esc or empty ÔåÆ cd ..
        if (-not $selected) {
            $currentPath = (Get-Location).Path
            $displayPath = Format-DisplayPath -Path $currentPath
            Write-Host "[cdx] esc, going up" -ForegroundColor DarkGray
            $parent = Split-Path $currentPath -Parent
            if ($parent -and $parent -ne $currentPath) {
                Set-Location $parent
                continue
            } else {
                ShowResult
                return
            }
        }

        # Handle become outputs
        if ($selected -eq '__EXIT_CDX__') {
            ShowResult
            return
        }

        if ($selected -eq '__GOTO_HOME__') {
            Write-Host "[cdx] goto home" -ForegroundColor DarkGray
            Set-Location $env:USERPROFILE
            continue
        }

        # Ctrl+Enter ÔåÆ cd + yazi
        if ($selected -like '__CD_YAZI__*') {
            $cleanSelected = $selected -replace '^__CD_YAZI__', '' -replace '^Ôÿà ', ''
            $fullPath = Join-Path $currentPath $cleanSelected
            if (Test-Path $fullPath -PathType Container) {
                Set-Location $fullPath
            } else {
                Set-Location (Split-Path $fullPath -Parent)
            }
            Write-Host "[cdx] yazi: $(Get-Location)" -ForegroundColor DarkGray
            yazi (Get-Location).Path
            ShowResult
            return
        }

        # Strip Ôÿà prefix
        $cleanSelected = $selected -replace '^Ôÿà ', ''

        if ($rgMode) {
            Write-Host "[cdx] open file: $cleanSelected" -ForegroundColor DarkGray
            $fullPath = $cleanSelected
            if ($hasBat) {
                bat $fullPath
            } else {
                Get-Content $fullPath -TotalCount 50
            }
        } else {
            $targetPath = Join-Path $currentPath $cleanSelected
            Write-Host "[cdx] cd: $targetPath" -ForegroundColor DarkGray
            Set-Location $targetPath
        }
    }
}

# ============================
# Invoke-CdxSearch (antes -s, ahora -g)
# ============================
function Invoke-CdxSearch {
    param([string]$Query)

    if (-not (Get-Command rg.exe -ErrorAction SilentlyContinue)) {
        Write-Host "[!] ripgrep not found. Install: winget install BurntSushi.ripgrep.MSVC" -ForegroundColor Red
        return
    }
    if (-not (Get-Command fzf.exe -ErrorAction SilentlyContinue)) {
        Write-Host "[!] fzf not found. Install: winget install junegunn.fzf" -ForegroundColor Red
        return
    }

    $priorityRoots = @(
        (Join-Path $HOME 'dev'),
        (Join-Path $HOME '.config')
    )
    $maxPriorityDepth = 6
    $maxSecondaryDepth = 5

    $excludeGlobs = @()
    foreach ($d in $script:ExcludeDirs) { $excludeGlobs += '--glob'; $excludeGlobs += "!$d" }
    foreach ($d in $script:ExcludeWinDirs) { $excludeGlobs += '--glob'; $excludeGlobs += "!$d" }
    foreach ($p in $script:ExcludePathGlobs) { $excludeGlobs += '--glob'; $excludeGlobs += "!$p" }

    Write-Host "[i] Searching for '$Query'..." -ForegroundColor Cyan

    # Phase 1: Content matches
    $contentMatches = @()
    foreach ($root in $priorityRoots) {
        if (Test-Path $root) {
            $contentMatches += & rg --files-with-matches --smart-case --hidden --max-depth $maxPriorityDepth @excludeGlobs $Query $root 2>$null
        }
    }
    $contentMatches += & rg --files-with-matches --smart-case --hidden --max-depth $maxSecondaryDepth `
        @excludeGlobs $Query $HOME 2>$null

    # Phase 2: File name matches
    $nameMatches = @()
    foreach ($root in $priorityRoots) {
        if (Test-Path $root) {
            $results = & rg --files --hidden --max-depth $maxPriorityDepth @excludeGlobs $root 2>$null
            if ($results) {
                $nameMatches += $results | & rg --smart-case $Query 2>$null
            }
        }
    }
    $results = & rg --files --hidden --max-depth $maxSecondaryDepth @excludeGlobs $HOME 2>$null
    if ($results) {
        $nameMatches += $results | & rg --smart-case $Query 2>$null
    }

    # Phase 3: Directory name matches
    $dirMatches = @()
    foreach ($root in $priorityRoots) {
        if (Test-Path $root) {
            $dirMatches += Get-ChildItem $root -Directory -Recurse -Depth $maxPriorityDepth -Force -ErrorAction SilentlyContinue |
                Where-Object { ($_.FullName -like "*$Query*") -and ($script:ExcludeDirs -notcontains $_.Name) } |
                Select-Object -ExpandProperty FullName
        }
    }
    $excludeDirNames = $script:ExcludeDirs + $script:ExcludeWinDirs
    $dirMatches += Get-ChildItem $HOME -Directory -Recurse -Depth $maxSecondaryDepth -Force -ErrorAction SilentlyContinue |
        Where-Object { ($_.FullName -like "*$Query*") -and ($excludeDirNames -notcontains $_.Name) } |
        Select-Object -ExpandProperty FullName

    # Combine & deduplicate
    $allResults = @($contentMatches) + @($nameMatches) + @($dirMatches) |
        Where-Object { $_ } |
        Select-Object -Unique

    if ((-not $allResults) -or ($allResults.Count -eq 0)) {
        Write-Host "[i] No matches found for '$Query'" -ForegroundColor Yellow
        return
    }

    Write-Host "[i] Found $($allResults.Count) matches. Select with fzf..." -ForegroundColor Cyan

    $safeQuery = $Query -replace '"', '""'
    $env:FZF_DEFAULT_OPTS = "--height=60% --layout=reverse --border --preview-window=up:50%"
    $selected = $allResults | fzf `
        --header="cdx search: Enter=navigate, Esc=cancel" `
        --preview "rg --context=3 --color=always --max-columns=200 `"$safeQuery`" {} 2>nul"

    if (-not $selected) { return }

    Resolve-CdxDestination -Path $selected
}

# ============================
# Resolve-CdxDestination
# ============================
function Resolve-CdxDestination {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        Write-Host "[!] Path no longer exists: $Path" -ForegroundColor Red
        return
    }

    if (Test-Path -PathType Container $Path) {
        Write-Host "[OK] cd $Path" -ForegroundColor Green
        Set-Location $Path
        return
    }

    $parent = Split-Path $Path -Parent
    Set-Location $parent

    if (Get-Command git -ErrorAction SilentlyContinue) {
        $gitRoot = & git -C $parent rev-parse --show-toplevel 2>$null
        if ($gitRoot) {
            Set-Location $gitRoot
            Write-Host "[OK] cd $gitRoot (git root)" -ForegroundColor Green
            return
        }
    }

    Write-Host "[OK] cd $parent" -ForegroundColor Green
}
