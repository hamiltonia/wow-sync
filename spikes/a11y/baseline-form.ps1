# Baseline: native Win32 controls (WinForms wraps the standard Win32 EDIT/BUTTON/LISTBOX classes).
# Run as its own process so UIA can be inspected from outside, like the GPUI app.
Add-Type -AssemblyName System.Windows.Forms
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Baseline'; $form.Width = 480; $form.Height = 520
$l = New-Object System.Windows.Forms.Label; $l.Text = 'A11y Baseline Heading'; $l.Top = 10; $l.Width = 300
$t = New-Object System.Windows.Forms.TextBox; $t.Top = 40; $t.TabIndex = 1
$b1 = New-Object System.Windows.Forms.Button; $b1.Text = 'Save'; $b1.Top = 70; $b1.TabIndex = 2
$b2 = New-Object System.Windows.Forms.Button; $b2.Text = 'Cancel'; $b2.Top = 70; $b2.Left = 100; $b2.TabIndex = 3
$c = New-Object System.Windows.Forms.CheckBox; $c.Text = 'Enable sync'; $c.Top = 100; $c.TabIndex = 4
$lb = New-Object System.Windows.Forms.ListBox; $lb.Top = 130; $lb.Height = 200; $lb.TabIndex = 5
1..10 | % { [void]$lb.Items.Add("List item $_") }
$form.Controls.AddRange(@($l, $t, $b1, $b2, $c, $lb))
[void]$form.ShowDialog()
