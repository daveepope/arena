using System.Threading.Tasks;
using System.Net;
using System.Net.Mail;

namespace ArenaExamples.Readings.Aspnet.Services;

public interface ISmtpClientService
{
    Task SendAsync(string to, string subject, string body);
}

public class SmtpClientService : ISmtpClientService
{
    private readonly string _host;
    private readonly int _port;

    public SmtpClientService(string host, int port)
    {
        _host = host;
        _port = port;
    }

    public async Task SendAsync(string to, string subject, string body)
    {
        try
        {
            var client = new SmtpClient(_host, _port)
            {
                EnableSsl = false,
                UseDefaultCredentials = false
            };
            var message = new MailMessage("device@arena.local", to)
            {
                Subject = subject,
                Body = body
            };
            await client.SendMailAsync(message);
        }
        catch
        {
        }
    }
}
