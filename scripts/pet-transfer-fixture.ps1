param([Parameter(Mandatory=$true)][string]$DataRoot,[switch]$BuildOnly)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$taskFixtureSource=@'
using System;
using System.IO;
using System.Drawing;
using System.Windows.Forms;
public class BuddyTransferFixture {
 [STAThread] public static void Main() {
  var here=Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly().Location);
  var p=new System.Diagnostics.ProcessStartInfo(Path.GetFullPath(Path.Combine(here,"../src-tauri/target/release/desktop-buddy.exe")),"--panel");
  p.UseShellExecute=false;p.EnvironmentVariables["BUDDY_DATA_DIR"]=Path.Combine(here,"pet-live-data");p.EnvironmentVariables["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"]="--remote-debugging-port=9225 --remote-debugging-address=127.0.0.1";
  var buddy=System.Diagnostics.Process.Start(p);
  try{Run(Path.Combine(here,"pet-transfer"));}finally{if(!buddy.HasExited)buddy.Kill();}
 }
 [STAThread] public static void Run(string root) {
  Directory.CreateDirectory(root);var input=Path.Combine(root,"input");var output=Path.Combine(root,"output");Directory.CreateDirectory(input);Directory.CreateDirectory(output);
  var files=new[]{Path.Combine(input,"handoff.txt"),Path.Combine(input,"\u732b.txt")};File.WriteAllText(files[0],"Buddy transfer fixture A");File.WriteAllText(files[1],"Buddy transfer fixture B");
  var form=new Form{Text="Buddy file handoff QA (test files only)",WindowState=FormWindowState.Maximized,AllowDrop=true,BackColor=Color.FromArgb(235,239,230)};
  var label=new Label{Text="QA FILE HANDOFF — NO USER FILES\nDrag the source button onto the pet. Then drag the loaded pet here to copy out.",Left=40,Top=40,Width=1000,Height=100,Font=new Font("Segoe UI",18)};
  var button=new Button{Text="Drag 2 fixture files",Left=60,Top=180,Width=260,Height=100,Font=new Font("Segoe UI",16)};
  button.MouseDown+=(s,e)=>{if(e.Button==MouseButtons.Left){var d=new DataObject(DataFormats.FileDrop,files);button.DoDragDrop(d,DragDropEffects.Copy);}};
  form.DragEnter+=(s,e)=>{e.Effect=e.Data.GetDataPresent(DataFormats.FileDrop)?DragDropEffects.Copy:DragDropEffects.None;};
  form.DragDrop+=(s,e)=>{var dropped=(string[])e.Data.GetData(DataFormats.FileDrop);foreach(var p in dropped){if(!Path.GetFullPath(p).StartsWith(Path.GetFullPath(input)+Path.DirectorySeparatorChar,StringComparison.OrdinalIgnoreCase))throw new Exception("Fixture accepts only its own input files");File.Copy(p,Path.Combine(output,Path.GetFileName(p)),true);}File.WriteAllLines(Path.Combine(root,"received.txt"),dropped);label.Text="COPY HANDOFF VERIFIED: "+dropped.Length+" files. Originals remain in input.";};
  form.Controls.Add(label);form.Controls.Add(button);Application.Run(form);
 }
}
'@
if($BuildOnly){Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -TypeDefinition $taskFixtureSource -OutputType WindowsApplication -OutputAssembly (Join-Path (Split-Path $PSScriptRoot) 'artifacts/buddy-pet-qa.exe');exit}
Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -TypeDefinition $taskFixtureSource
[BuddyTransferFixture]::Run($DataRoot)
