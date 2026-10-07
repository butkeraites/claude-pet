// Spike do Windows (M8): abre uma janela topmost sem borda, numa cor viva, e
// tira um screenshot do desktop inteiro. Se o screenshot mostrar a janela, o
// runner tem tela visível e o loop "compilar -> rodar -> screenshot -> eu vejo"
// fecha — o "Mac virtual" para conferir o overlay do Windows no CI.
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Threading;
using System.Windows.Forms;

static class Spike
{
    [STAThread]
    static void Main()
    {
        Application.EnableVisualStyles();

        var vs = SystemInformation.VirtualScreen;
        Console.WriteLine($"VirtualScreen: {vs.Width}x{vs.Height} em ({vs.X},{vs.Y})");

        var f = new Form
        {
            FormBorderStyle = FormBorderStyle.None,
            TopMost = true,
            StartPosition = FormStartPosition.Manual,
            Bounds = new Rectangle(vs.X + 200, vs.Y + 150, 320, 200),
            BackColor = Color.FromArgb(0xE0, 0x32, 0xC8),
            ShowInTaskbar = false,
        };
        f.Controls.Add(new Label
        {
            Text = "ZECA\nWINDOWS\nSPIKE",
            Dock = DockStyle.Fill,
            TextAlign = ContentAlignment.MiddleCenter,
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 24, FontStyle.Bold),
        });

        f.Show();
        f.Activate();
        f.Refresh();
        Application.DoEvents();
        Thread.Sleep(1500);
        Application.DoEvents();

        Console.WriteLine($"janela visível: {f.Visible}, bounds: {f.Bounds}");

        using (var bmp = new Bitmap(vs.Width, vs.Height, PixelFormat.Format32bppArgb))
        {
            using (var g = Graphics.FromImage(bmp))
                g.CopyFromScreen(vs.X, vs.Y, 0, 0, vs.Size, CopyPixelOperation.SourceCopy);
            bmp.Save("screenshot.png", ImageFormat.Png);
        }
        Console.WriteLine("salvei screenshot.png");
    }
}
