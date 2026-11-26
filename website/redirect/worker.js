// The docs used to live at matenadar-docs.levships.com. Every path there now
// lives at the same path on matenadar.levships.com.
export default {
	fetch(request) {
		const from = new URL(request.url);
		const to = new URL(from.pathname + from.search + from.hash, 'https://matenadar.levships.com');
		return Response.redirect(to.href, 301);
	},
};
