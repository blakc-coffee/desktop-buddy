param([int]$Samples=15)
$ErrorActionPreference='Stop'
$taskProject=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $taskProject
$taskExe=Join-Path $taskProject 'src-tauri/target/release/desktop-buddy.exe'
$env:BUDDY_DATA_DIR=Join-Path $taskProject 'artifacts/benchmark-data'
Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class BuddyMemory {
 [StructLayout(LayoutKind.Sequential)] public struct Counters {
  public uint cb, PageFaultCount;
  public UIntPtr PeakWorkingSetSize,WorkingSetSize,QuotaPeakPagedPoolUsage,QuotaPagedPoolUsage,QuotaPeakNonPagedPoolUsage,QuotaNonPagedPoolUsage,PagefileUsage,PeakPagefileUsage,PrivateUsage,PrivateWorkingSetSize;
  public ulong SharedCommitUsage;
 }
 [DllImport("psapi.dll",SetLastError=true)] public static extern bool GetProcessMemoryInfo(IntPtr hProcess,ref Counters counters,uint cb);
 public static long[] Read(IntPtr handle){var c=new Counters();c.cb=(uint)Marshal.SizeOf<Counters>();if(!GetProcessMemoryInfo(handle,ref c,c.cb))return null;return new[]{(long)c.PrivateWorkingSetSize.ToUInt64(),(long)c.PrivateUsage.ToUInt64(),(long)c.WorkingSetSize.ToUInt64()};}
}
'@
function Get-TaskTree([int]$RootId) {
 $all=Get-CimInstance Win32_Process -Filter "Name='desktop-buddy.exe' OR Name='msedgewebview2.exe'"
 $ids=[System.Collections.Generic.HashSet[int]]::new();[void]$ids.Add($RootId)
 do {$added=$false;foreach($p in $all){if($ids.Contains([int]$p.ParentProcessId)-and -not $ids.Contains([int]$p.ProcessId)){[void]$ids.Add([int]$p.ProcessId);$added=$true}}}while($added)
 return @($ids)
}
function Measure-TaskPhase([string]$Phase,[int]$RootId){
 if(!(Get-Process -Id $RootId -ErrorAction SilentlyContinue)){throw "Buddy exited before $Phase"}
 $ids=Get-TaskTree $RootId;$rows=@();$previous=@{};$last=[datetime]::UtcNow
 for($n=0;$n -le $Samples;$n++){
  $stamp=[datetime]::UtcNow;$seconds=($stamp-$last).TotalSeconds;$cpuDelta=0.0;$privateWs=0L;$commit=0L;$totalWs=0L;$members=0;$valid=$true
  foreach($processId in $ids){$p=Get-Process -Id $processId -ErrorAction SilentlyContinue;if(!$p){continue};$members++;$cpu=$p.CPU;if($previous.ContainsKey($processId)){$cpuDelta+=[math]::Max(0,$cpu-$previous[$processId])};$previous[$processId]=$cpu;$mem=[BuddyMemory]::Read($p.Handle);if($null -eq $mem){$valid=$false;$commit+=$p.PrivateMemorySize64;$totalWs+=$p.WorkingSet64}else{$privateWs+=$mem[0];$commit+=$mem[1];$totalWs+=$mem[2]}}
  if($n -gt 0){$rows+=[pscustomobject]@{phase=$Phase;sample=$n;processes=$members;privateWorkingSetMiB=if($valid){[math]::Round($privateWs/1MB,2)}else{$null};privateCommitMiB=[math]::Round($commit/1MB,2);sumWorkingSetMiB=[math]::Round($totalWs/1MB,2);cpuPercent=[math]::Round($cpuDelta/($seconds*[Environment]::ProcessorCount)*100,3)}}
  $last=$stamp;Start-Sleep -Milliseconds 1000
 };return $rows
}
$taskProcess=Start-Process -FilePath $taskExe -ArgumentList '--tray' -PassThru -WindowStyle Hidden
try {
 Start-Sleep -Seconds 10
 $tray=Measure-TaskPhase 'tray-only' $taskProcess.Id
 $petLaunch=Start-Process -FilePath $taskExe -PassThru -WindowStyle Hidden
 $petLaunch.WaitForExit(5000)|Out-Null
 Start-Sleep -Seconds 10
 $pet=Measure-TaskPhase 'pet-only' $taskProcess.Id
 $barLaunch=Start-Process -FilePath $taskExe -ArgumentList '--commands' -PassThru -WindowStyle Hidden
 $barLaunch.WaitForExit(5000)|Out-Null
 Start-Sleep -Seconds 10
 $bar=Measure-TaskPhase 'command-bar-open' $taskProcess.Id
 $barClose=Start-Process -FilePath $taskExe -ArgumentList '--close-commands' -PassThru -WindowStyle Hidden
 $barClose.WaitForExit(5000)|Out-Null
 $open=Start-Process -FilePath $taskExe -ArgumentList '--panel' -PassThru -WindowStyle Hidden;$open.WaitForExit(5000)|Out-Null
 Start-Sleep -Seconds 10
 $panel=Measure-TaskPhase 'panel-open' $taskProcess.Id
 $close=Start-Process -FilePath $taskExe -ArgumentList '--close-panel' -PassThru -WindowStyle Hidden;$close.WaitForExit(5000)|Out-Null
 Start-Sleep -Seconds 10
 $closed=Measure-TaskPhase 'panel-destroyed' $taskProcess.Id
 $rows=@($tray)+@($pet)+@($bar)+@($panel)+@($closed)
 $rows | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath 'artifacts/memory-samples.json' -Encoding utf8
 $summary=$rows|Group-Object phase|ForEach-Object{[pscustomobject]@{phase=$_.Name;samples=$_.Count;processes=($_.Group.processes|Measure-Object -Maximum).Maximum;meanPrivateWorkingSetMiB=if(@($_.Group|Where-Object {$null -ne $_.privateWorkingSetMiB}).Count -eq $_.Count){[math]::Round(($_.Group.privateWorkingSetMiB|Measure-Object -Average).Average,2)}else{$null};meanPrivateCommitMiB=[math]::Round(($_.Group.privateCommitMiB|Measure-Object -Average).Average,2);meanCPUPercent=[math]::Round(($_.Group.cpuPercent|Measure-Object -Average).Average,3)}}
 $summary|ConvertTo-Json -Depth 4|Set-Content -LiteralPath 'artifacts/memory-summary.json' -Encoding utf8
 $summary|Format-Table
}finally{if(!$taskProcess.HasExited){$quit=Start-Process -FilePath $taskExe -ArgumentList '--quit' -PassThru -WindowStyle Hidden;$quit.WaitForExit(5000)|Out-Null;if(!$taskProcess.WaitForExit(5000)){Stop-Process -Id $taskProcess.Id}}}

