<#
    Comprehensive CLI Edge Case Test Suite
    Run from the workspace root: ./test_cli_all_commands.ps1
    Calls the compiled binary directly for reliable argument passing.
#>

$ErrorActionPreference = "Stop"
$script:Passed = 0
$script:Failed = 0

function Write-TestHeader([string]$Title) {
    Write-Host ""
    Write-Host "══════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host "  $Title" -ForegroundColor Cyan
    Write-Host "══════════════════════════════════════════════════════════" -ForegroundColor Cyan
}

function Invoke-Cli([string[]]$Args) {
    $bin = "target\debug\displaymanager_cli.exe"
    if (-not (Test-Path $bin)) { throw "Binary not found at $bin. Build first." }
    $g = [guid]::NewGuid().ToString("N")
    $outFile = "$env:TEMP\cli_out_$g.txt"
    $errFile = "$env:TEMP\cli_err_$g.txt"
    $argStr = $Args -join ' '
    $p = Start-Process -FilePath $bin -ArgumentList $argStr -NoNewWindow -Wait -PassThru -RedirectStandardOutput $outFile -RedirectStandardError $errFile
    $stdout = ""
    $stderr = ""
    if (Test-Path $outFile) { $stdout = [System.IO.File]::ReadAllText($outFile, [System.Text.Encoding]::UTF8); Remove-Item $outFile -ErrorAction SilentlyContinue }
    if (Test-Path $errFile) { $stderr = [System.IO.File]::ReadAllText($errFile, [System.Text.Encoding]::UTF8); Remove-Item $errFile -ErrorAction SilentlyContinue }
    return @{ ExitCode = $p.ExitCode; Stdout = $stdout; Stderr = $stderr }
}

function Test-Case {
    param(
        [string]$Name,
        [string[]]$Args,
        [int]$ExpectedExitCode = 0,
        [scriptblock]$StdoutCheck,
        [scriptblock]$StderrCheck
    )
    $result = Invoke-Cli -Args $Args
    $passed = $true; $failures = @()
    if ($result.ExitCode -ne $ExpectedExitCode) {
        $passed = $false
        $failures += "Exit code mismatch: expected $ExpectedExitCode, got $($result.ExitCode)"
    }
    if ($StdoutCheck) {
        try { & $StdoutCheck $result.Stdout | Out-Null }
        catch { $passed = $false; $failures += "Stdout check failed: $_" }
    }
    if ($StderrCheck) {
        try { & $StderrCheck $result.Stderr | Out-Null }
        catch { $passed = $false; $failures += "Stderr check failed: $_" }
    }
    if ($passed) { $script:Passed++; $status = "PASS"; $color = "Green" }
    else { $script:Failed++; $status = "FAIL"; $color = "Red" }
    Write-Host "[$status] $Name" -ForegroundColor $color
    if (-not $passed) {
        foreach ($f in $failures) { Write-Host "      -> $f" -ForegroundColor Red }
        if ($result.Stdout) { Write-Host "      STDOUT: $($result.Stdout.Substring(0, [Math]::Min(300, $result.Stdout.Length)))" -ForegroundColor DarkGray }
        if ($result.Stderr) { Write-Host "      STDERR: $($result.Stderr.Substring(0, [Math]::Min(300, $result.Stderr.Length)))" -ForegroundColor DarkGray }
    }
}

function Assert-Contains([string]$Hay, [string]$Needle) {
    if ($Hay -notmatch [regex]::Escape($Needle)) { throw "Does not contain '$Needle'" }
}

# Pre-flight build
Write-Host "Building CLI binary..." -ForegroundColor Yellow
$build = Start-Process -FilePath "cargo" -ArgumentList "build","--quiet","--bin","displaymanager_cli" -NoNewWindow -Wait -PassThru
if ($build.ExitCode -ne 0) { Write-Host "Build failed." -ForegroundColor Red; exit 1 }
Write-Host "Build OK" -ForegroundColor Green

# ─── 1. Help & Version ───
Write-TestHeader "1. Help & Version"
Test-Case -Name "main --help"          -Args @("--help")          -ExpectedExitCode 2
Test-Case -Name "main --version"       -Args @("--version")       -ExpectedExitCode 2
Test-Case -Name "display --help"       -Args @("display","--help") -ExpectedExitCode 2
Test-Case -Name "ddc --help"           -Args @("ddc","--help")     -ExpectedExitCode 2

# ─── 2. Display Scan ───
Write-TestHeader "2. Display Scan"
Test-Case -Name "display scan (text)"  -Args @("display","scan") -ExpectedExitCode 0 -StdoutCheck { Assert-Contains $args[0] "Monitor" }
Test-Case -Name "display scan --json"  -Args @("display","scan","--json") -ExpectedExitCode 0 -StdoutCheck { Assert-Contains $args[0] "target_id" }
Test-Case -Name "display scan --edid-json file" -Args @("display","scan","--edid-json","$env:TEMP\edid_out.json") -StdoutCheck {
    if (-not (Test-Path "$env:TEMP\edid_out.json")) { throw "File not created" }
}
Test-Case -Name "display scan --edid-json ." -Args @("display","scan","--edid-json","edid_test_dump.json") -StdoutCheck {
    if (-not (Test-Path "edid_test_dump.json")) { throw "File not created" }
}
$sp = "$env:TEMP\cli test spaces.json"
Test-Case -Name "display scan --edid-json spaces" -Args @("display","scan","--edid-json",$sp) -StdoutCheck {
    if (-not (Test-Path $sp)) { throw "File with spaces not created" }
}
Test-Case -Name "display scan both flags" -Args @("display","scan","--json","--edid-json","$env:TEMP\both.json")
Test-Case -Name "display scan plain"      -Args @("display","scan")

# ─── 3. Display Info ───
Write-TestHeader "3. Display Info"
Test-Case -Name "info missing --output"               -Args @("display","info")
Test-Case -Name "info bogus-name"                     -Args @("display","info","--output","__nonexistent__")
Test-Case -Name "info empty string"                   -Args @("display","info","--output","")
Test-Case -Name "info 999999"                         -Args @("display","info","--output","999999")
Test-Case -Name "info --json bogus"                   -Args @("display","info","--output","bogus","--json")

# ─── 4. Display Set ───
Write-TestHeader "4. Display Set"
Test-Case -Name "set missing --output"               -Args @("display","set")
Test-Case -Name "set --verify-only bogus"            -Args @("display","set","--output","nonexistent","--verify-only")
Test-Case -Name "set --mode-type off bogus"          -Args @("display","set","--output","bogus","--mode-type","off")
Test-Case -Name "set --mode-type invalidvalue"       -Args @("display","set","--output","0","--mode-type","invalidvalue")
Test-Case -Name "set --mode-type cloned no-from"     -Args @("display","set","--output","0","--mode-type","cloned")
Test-Case -Name "set --rotate 45 invalid"            -Args @("display","set","--output","0","--rotate","45")
foreach ($rot in @("0","90","180","270")) {
    Test-Case -Name "set --rotate $rot valid"        -Args @("display","set","--output","0","--rotate",$rot)
}
Test-Case -Name "set --mode abc invalid"             -Args @("display","set","--output","0","--mode","abc")
Test-Case -Name "set --hdr maybe invalid"            -Args @("display","set","--output","0","--hdr","maybe")
Test-Case -Name "set --scale 0.1 too low"            -Args @("display","set","--output","0","--scale","0.1")
Test-Case -Name "set --scale 6.0 too high"           -Args @("display","set","--output","0","--scale","6.0")
Test-Case -Name "set --scale 0.25 lower"             -Args @("display","set","--output","0","--scale","0.25")
Test-Case -Name "set --scale 5.0 upper"              -Args @("display","set","--output","0","--scale","5.0")
Test-Case -Name "set --pos garbage"                  -Args @("display","set","--output","0","--pos","not_a_number")
Test-Case -Name "set --pos 100,200"                  -Args @("display","set","--output","0","--pos","100,200")
Test-Case -Name "set --pos 100x200"                  -Args @("display","set","--output","0","--pos","100x200")
Test-Case -Name "set --auto-pos bogus"               -Args @("display","set","--output","bogus","--auto-pos")
Test-Case -Name "set --ccd-wake bogus"               -Args @("display","set","--output","bogus","--ccd-wake")
Test-Case -Name "set --refresh-rate 60000 bogus"     -Args @("display","set","--output","bogus","--refresh-rate","60000")

# ─── 5. DDC List ───
Write-TestHeader "5. DDC List"
Test-Case -Name "ddc list"          -Args @("ddc","list")
Test-Case -Name "ddc list --json"   -Args @("ddc","list","--json")
Test-Case -Name "ddc --id 999 brightness" -Args @("ddc","--id","999","brightness","50")
Test-Case -Name "ddc --id -1 list"  -Args @("ddc","--id","-1","list")

# ─── 6. DDC Actions ───
Write-TestHeader "6. DDC Actions"
foreach ($a in @("brightness","contrast","volume")) {
    Test-Case -Name "ddc $a 50" -Args @("ddc",$a,"50")
}

# ─── 7. DDC Power ───
Write-TestHeader "7. DDC Power"
Test-Case -Name "ddc power on"  -Args @("ddc","power","on")
Test-Case -Name "ddc power off" -Args @("ddc","power","off")
Test-Case -Name "ddc power 1"   -Args @("ddc","power","1")
Test-Case -Name "ddc power 0"   -Args @("ddc","power","0")
Test-Case -Name "ddc power maybe invalid" -Args @("ddc","power","maybe")

# ─── 8. DDC Input ───
Write-TestHeader "8. DDC Input"
$validIn = @("dp1","displayport1","displayport1.0","dp2","displayport2","hdmi1","hdmi-1","hdmi1.0","hdmi2","hdmi-2","0x0F","0x10","0x11","0x12")
foreach ($i in $validIn) { Test-Case -Name "input $i valid" -Args @("ddc","input",$i) }
$invalidIn = @("vga","dvi","usbc","abc","0xFF","0xZZ","displayport3","dp3","hdmi3","dp")
foreach ($i in $invalidIn) { Test-Case -Name "input $i invalid" -Args @("ddc","input",$i) }

# ─── 9. Color Gains ───
Write-TestHeader "9. Color Gains"
Test-Case -Name "ddc color-gains 50 50 50" -Args @("ddc","color-gains","50","50","50")

# ─── 10. Empty / Whitespace ───
Write-TestHeader "10. Empty / Whitespace"
Test-Case -Name "info --output '   '" -Args @("display","info","--output","   ")

# ─── 11. File System ───
Write-TestHeader "11. File System"
$longFn = "$env:TEMP\edid_" + ("x" * 200) + ".json"
Test-Case -Name "scan --edid-json long filename" -Args @("display","scan","--edid-json",$longFn)

# ─── 12. Combinations ───
Write-TestHeader "12. Combinations"
Test-Case -Name "scan twice 1" -Args @("display","scan")
Test-Case -Name "scan twice 2" -Args @("display","scan")
Test-Case -Name "scan --json"  -Args @("display","scan","--json")

# ─── 13. Error Propagation ───
Write-TestHeader "13. Error Propagation"
Test-Case -Name "set --hdr ENABLE" -Args @("display","set","--output","0","--hdr","ENABLE")
Test-Case -Name "set --hdr enable" -Args @("display","set","--output","0","--hdr","enable")
Test-Case -Name "set --hdr off"    -Args @("display","set","--output","0","--hdr","off")
Test-Case -Name "set --rotate Nine0" -Args @("display","set","--output","0","--rotate","Nine0")
Test-Case -Name "set --mode 1920xabc" -Args @("display","set","--output","0","--mode","1920xabc")

# ─── Summary ───
Write-Host ""
Write-Host "══════════════════════════════════════════════════════════" -ForegroundColor Yellow
$total = $script:Passed + $script:Failed
$pct = if ($total -gt 0) { [math]::Round(($script:Passed / $total) * 100, 1) } else { 0 }
Write-Host "  SUMMARY: $script:Passed passed, $script:Failed failed ($pct%) of $total tests" -ForegroundColor Yellow
Write-Host "══════════════════════════════════════════════════════════" -ForegroundColor Yellow
if ($script:Failed -gt 0) { Write-Host "Some tests failed. Review failures above." -ForegroundColor Red; exit 1 }
else { Write-Host "All tests passed!" -ForegroundColor Green; exit 0 }